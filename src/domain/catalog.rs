use super::{DataLayout, SourceRegistry, WeatherElement, WeatherModelId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PolicyContext {
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedElement {
    pub element: WeatherElement,
    pub model: WeatherModelId,
    pub variable_name: &'static str,
}

pub trait ModelPriorityPolicy: Send + Sync {
    fn priority(&self, context: &PolicyContext, element: WeatherElement) -> Vec<WeatherModelId>;
}

#[derive(Debug, Clone)]
pub struct RuleBasedModelPriorityPolicy {
    default_priority: Vec<WeatherModelId>,
    rules: Vec<ModelPriorityRule>,
}

#[derive(Debug, Clone)]
pub struct ModelPriorityRule {
    pub tag: String,
    pub priority: Vec<WeatherModelId>,
}

impl RuleBasedModelPriorityPolicy {
    pub fn new(default_priority: Vec<WeatherModelId>, rules: Vec<ModelPriorityRule>) -> Self {
        Self {
            default_priority,
            rules,
        }
    }
}

impl Default for RuleBasedModelPriorityPolicy {
    fn default() -> Self {
        Self::new(
            vec![
                WeatherModelId::EcmwfIfs,
                WeatherModelId::EcmwfIfs025,
                WeatherModelId::Gfs025,
                WeatherModelId::DwdIcon,
            ],
            Vec::new(),
        )
    }
}

impl ModelPriorityPolicy for RuleBasedModelPriorityPolicy {
    fn priority(&self, context: &PolicyContext, _element: WeatherElement) -> Vec<WeatherModelId> {
        self.rules
            .iter()
            .find(|rule| context.tags.iter().any(|tag| tag == &rule.tag))
            .map(|rule| rule.priority.clone())
            .unwrap_or_else(|| self.default_priority.clone())
    }
}

pub struct WeatherElementCatalog<P = RuleBasedModelPriorityPolicy> {
    policy: P,
}

impl<P> WeatherElementCatalog<P>
where
    P: ModelPriorityPolicy,
{
    pub fn new(policy: P) -> Self {
        Self { policy }
    }

    pub fn resolve(
        &self,
        registry: &SourceRegistry,
        context: &PolicyContext,
        layout: DataLayout,
        element: WeatherElement,
    ) -> Option<ResolvedElement> {
        self.policy
            .priority(context, element)
            .into_iter()
            .find_map(|model| {
                let source = registry.get(model)?;
                let variable_name = source.variable_name(layout, element)?;
                Some(ResolvedElement {
                    element,
                    model,
                    variable_name,
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::open_meteo;

    #[test]
    fn falls_back_using_injected_policy_rule() {
        let registry = open_meteo::OpenMeteoSources.registry();
        let catalog = WeatherElementCatalog::new(RuleBasedModelPriorityPolicy::new(
            vec![
                WeatherModelId::EcmwfIfs,
                WeatherModelId::EcmwfIfs025,
                WeatherModelId::Gfs025,
                WeatherModelId::DwdIcon,
            ],
            vec![ModelPriorityRule {
                tag: "example:dwd-second".to_string(),
                priority: vec![
                    WeatherModelId::EcmwfIfs025,
                    WeatherModelId::DwdIcon,
                    WeatherModelId::Gfs025,
                ],
            }],
        ));

        let resolved = catalog
            .resolve(
                &registry,
                &PolicyContext {
                    tags: vec!["example:dwd-second".to_string()],
                },
                DataLayout::Spatial,
                WeatherElement::WeatherCode,
            )
            .expect("resolved element");
        assert_eq!(resolved.model, WeatherModelId::DwdIcon);
        assert_eq!(resolved.variable_name, "weather_code");
    }
}
