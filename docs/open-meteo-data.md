# Open-Meteo 数据目录

权威来源：[open-meteo/open-data](https://github.com/open-meteo/open-data) · 桶 `s3://openmeteo` · `https://openmeteo.s3.amazonaws.com`

本文档说明 **三种数据模式各自包含哪些气象要素（variable）**。要素名即 S3 / `.om` 文件内使用的字符串（如 `temperature_2m`、`precipitation`）。

完整清单（按类别分组 + 字母序 txt）见 [`data/variables/`](data/variables/)。快照日期：2026-06-07。

---

## 1. 三种数据模式

同一模型、同一批预报，要素**语义相同**，但**组织方式**不同：

| 模式 | S3 前缀 | 文件组织 | 要素清单从哪查 | AWS 保留（官方） |
|------|---------|----------|----------------|------------------|
| **Spatial** | `data_spatial/` | 每个 **时刻** 一个 `.om`，内含**多要素**栅格 | `{model}/latest.json` → `variables` | ~7 天 |
| **Run** | `data_run/` | 每个 **要素** 一个 `.om`，内含该 run **全时段** | `{model}/latest.json` → `variables` | ~3 个月 |
| **Timeseries** | `data/` | 每个 **要素** 一条滚动序列，按 chunk 分文件 | 列 `data/{model}/` 子目录名 | 长期滚动 |

### 1.1 路径约定

```
data_spatial/{model}/{YYYY}/{MM}/{DD}/{hhmm}Z/{YYYY-MM-DDThhmm}.om
data_run/{model}/{YYYY}/{MM}/{DD}/{hhmm}Z/{variable}.om
data/{model}/{variable}/{chunk}.om
```

Manifest：

```
data_spatial/{model}/latest.json    # spatial 当前 run（官方主入口）
data_run/{model}/latest.json        # run 归档当前 run
data_spatial/{model}/in-progress.json   # run 进行中
```

### 1.2 读要素时的注意点（官方）

- **原生时间步长**：spatial / run 用模式原生步长（如 ECMWF 为 **3 小时**），不会像 `data/` 那样插值到更高频率。
- **累积/平均量**（`precipitation`、辐射等）：**第一个 valid time 常无值**。
- **风速**：S3 存 `wind_u_component_10m` / `wind_v_component_10m`；Open-Meteo **API** 可算 `wind_speed_10m`，桶里不一定有后者这个名字。
- **单文件多要素**：spatial 每个 `.om` 含该时刻大量要素；应用内读 metadata 确认实际有哪些 child variable。
- **延迟要素**：部分模型有 `*_model-level.om` 与 `latest_model-level.json`。

---

## 2. 模型时空特征（官方摘要）

| 模型 ID | 区域 | 分辨率 | 时间步长 | 预报长度 | 更新 |
|---------|------|--------|----------|----------|------|
| `ecmwf_ifs025` | 全球 | 0.25° | 3 h | 15 天 | 6 h |
| `ncep_gfs025` | 全球 | 0.25° | 1 h | 16 天 | 6 h |
| `dwd_icon` | 全球 | 0.1° | 1 h | 7.5 天 | 6 h |

更多模型见 [官方模型表](https://github.com/open-meteo/open-data#weather-forecast-models)。

---

## 3. 各模式要素数量（当前 run 快照）

| 模型 | Spatial manifest | Run manifest | Timeseries 索引 |
|------|------------------:|-------------:|----------------:|
| `ecmwf_ifs025` | 119 | 114 | 158 |
| `ncep_gfs025` | 316 | 105 | 317 |
| `dwd_icon` | 123 | 107 | 134 |

说明：

- **Timeseries** 通常最多（含 `divergence_of_wind_*`、`static` 等 extra 要素）。
- **Run manifest** 往往是当前 run 的**精简**要素集（尤其 GFS：run 105 个 vs spatial manifest 316 个——spatial manifest 会列出更多气压层）。
- **单个 spatial `.om` 文件**内的要素以文件 metadata 为准；与 manifest 大体一致，但随 run 可能略有增减。

---

## 4. 常用地面要素对照

下列为业务常关心的要素；✓ 表示在该模型 **run manifest**（`data_run/{model}/latest.json`）中出现。

| 要素 | 含义 | ecmwf_ifs025 | ncep_gfs025 | dwd_icon |
|------|------|:------------:|:-----------:|:--------:|
| `temperature_2m` | 2 m 气温 | ✓ | — | ✓ |
| `temperature_2m_min` / `_max` | 日 min/max | ✓ | — | — |
| `relative_humidity_2m` | 2 m 相对湿度 | ✓ | — | ✓ |
| `precipitation` | 降水量 | ✓ | — | ✓ |
| `rain` | 雨量 | — | — | ✓ |
| `showers` | 阵雨 | — | — | ✓ |
| `snow_depth` | 雪深 | ✓ | — | ✓ |
| `snowfall_water_equivalent` | 降雪（水当量） | ✓ | — | ✓ |
| `cloud_cover` | 总云量 | ✓ | — | ✓ |
| `cloud_cover_low` / `_mid` / `_high` | 分层云 | ✓ | — | ✓ |
| `shortwave_radiation` | 短波辐射 | ✓ | — | — |
| `direct_radiation` / `diffuse_radiation` | 直射/散射 | — | — | ✓ |
| `wind_u_component_10m` / `wind_v_component_10m` | 10 m 风分量 | ✓ | — | ✓ |
| `wind_gusts_10m` | 10 m 阵风 | ✓ | ✓ | ✓ |
| `cape` | 对流有效位能 | ✓ | ✓ | ✓ |
| `visibility` | 能见度 | — | ✓ | — |
| `weather_code` | 天气代码 | — | — | ✓ |
| `freezing_level_height` | 冻结层高度 | — | ✓ | ✓ |
| `lifted_index` | 抬升指数 | — | ✓ | — |

「—」表示不在该模型当前 run manifest 中；**不代表** spatial 单文件中绝对不存在（例如 GFS spatial 文件结构以 manifest / metadata 为准）。

### 4.1 ECMWF IFS 0.25° — run 模式要素分类（114 个）

| 类别 | 数量 | 代表要素 |
|------|-----:|----------|
| 气压层风 | 26 | `wind_u_component_850hPa`, … |
| 相对湿度 | 14 | `relative_humidity_2m`, `relative_humidity_500hPa`, … |
| 位势高度 | 13 | `geopotential_height_500hPa`, … |
| 垂直速度 | 13 | `vertical_velocity_500hPa`, … |
| 气压层温度 | 13 | `temperature_850hPa`, … |
| 土壤 | 8 | `soil_temperature_0_to_7cm`, `soil_moisture_0_to_7cm`, … |
| 近地面/高度风 | 5 | `wind_u_component_10m`, `wind_gusts_10m`, … |
| 云 | 4 | `cloud_cover`, `cloud_cover_low`, … |
| 近地面温度 | 4 | `temperature_2m`, `temperature_2m_min`, … |
| 降水 | 3 | `precipitation`, `precipitation_type`, `runoff` |
| 雪 | 3 | `snow_depth`, `snowfall_water_equivalent`, … |
| 其他 | 若干 | `cape`, `shortwave_radiation`, `ocean_u_current`, … |

GFS / DWD 全部分类见 [`data/variables/CATALOG.md`](data/variables/CATALOG.md)。

---

## 5. 要素清单文件

| 文件 | 内容 |
|------|------|
| `{model}-spatial.txt` | Spatial manifest 要素名（字母序） |
| `{model}-run.txt` | Run manifest 要素名 |
| `{model}-timeseries.txt` | Timeseries 索引要素名 |
| `CATALOG.md` | 三模型 × 三模式，按类别分组 |
| `manifest.json` | 快照元数据（数量、reference_time） |

模型 ID：`ecmwf_ifs025`、`ncep_gfs025`、`dwd_icon`。

---

## 6. 更新快照

从仓库根目录执行（需网络）：

```sh
cd om-server/docs/data/variables
# 重新拉取 latest.json 与 timeseries 前缀，覆盖 txt / manifest.json
# （可与 om-server 维护脚本对齐；当前为 2026-06-07 手工快照）
```

在线浏览全部模型与变量：[Open-Meteo S3 Explorer](https://openmeteo.s3.amazonaws.com/index.html)（官方 README 链接）。

---

## 7. 相关链接

- 官方数据说明：[github.com/open-meteo/open-data](https://github.com/open-meteo/open-data)
- om-server 实现说明（sync / gRPC）：见仓库内实现，**不在本文档范围**
