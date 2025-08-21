use std::{
    fmt::Debug,
    sync::{Arc, atomic::AtomicI64},
};

use downcast_rs::{DowncastSync, impl_downcast};
use dyn_clone::{DynClone, clone_trait_object};
use hashbrown::HashMap;
use serde_json::Value;

/// 硬體設備連線設定
///
/// 實作本 trait 的 struct/enum 代表其定義了主程式連線至硬體時所需要的各項資訊
///
/// 實作 [`Connection`] trait 時，需要在 [`Connection::Config`] type ailas 指定一種有實作本 trait 的 struct/enum
///
/// - 當程式初始化時，程式會呼叫 [`Connection::init()`] function，並以實作本 trait 的 struct/enum 做為參數，供連線時使用。
/// - 當連線異常並達到一定次數時，程式會呼叫 [`Connection::reconnect()`] function 進行重新連線，實作本 trait 的 struct/enum 會作為參數，供重新連線時使用。
/// - 當連線資訊更新時，程式會呼叫 [`Connection::update_config()`] function，利用參數中帶入的另一同樣實作本 trait 的 struct/enum 開啓新的連線，並取代既有連線。
///
/// # 實作要求
///
/// 實作本 trait 的 struct/enum 必須同時實作 [`Debug`], [`Clone`], [`Send`] 和 [`Sync`] 四個 trait ，並持有 `'static` lifetime
///
/// - [`Debug`]：可以輸出偵錯用資訊
/// - [`Send`]：可以被傳送至其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - [`Sync`]：可以被分享給其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - `'static` lifetime：標記引用需要在程式運行期間均有效
///
/// # 範例
///
/// 定義 Modbus RTU 連線時，需要讓用戶指定調變速率（又稱鮑率 baud rate）、數據位（data bits）、同位（parity） 和停止位（stop bits），這時可以建立一個 struct 包含以上資訊，並實作本 trait ：
/// ```rust
/// #[derive(Debug)]
/// struct ExampleModbusConnectionConfig {
///     baud_rate: u32,
///     data_bits: String,
///     parity: String,
///     stop_bits: String
/// }
///
/// impl ConnectionConfig for ExampleModbusConnectionConfig {}
/// ```
pub trait ConnectionConfig: Debug + Send + Sync + 'static {}

/// 點位目標
///
/// 實作本 trait 的 struct/enum 代表點位，一個設備可能有多個點位（如 Modbus 設備可以有多個 Register，如果 Register 代表一個功能，即稱該 Register 為一個點位），實作 [`Connection`] trait 時，需要在 [`Connection::Target`] type ailas 指定一種有實作本 trait 的 struct/enum
///
/// 當程式初始化時，會呼叫 [`Connection::init_targets()`] function，並以實作本 trait 的 struct/enum 做為參數，供分類並將點位轉換為設備狀態請求（實作 [`DeviceStateRequest`] 並指定於 [`Connection::Request`] 的 struct/enum）時使用。
///
/// # 實作要求
///
/// 實作本 trait 的 struct/enum 必須同時實作 [`Debug`], [`Send`] 和 [`Sync`] 三個 trait 、持有 `'static` lifetime 且維持 [dyn-compatible](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
///
/// - [`Debug`]：可以輸出偵錯用資訊
/// - [`Send`]：可以被傳送至其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - [`Sync`]：可以被分享給其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - `'static` lifetime：標記引用需要在程式運行期間均有效
/// - dyn-compatible：要求實作後依然保持可以利用[動態分派 (dynamic dispatch)](https://zh.wikipedia.org/zh-tw/动态分派)
///
/// # 範例
/// Modbus RTU 點位被儲存於 JSON 格式的資料中，利用 [`serde_json::Value`] 型別儲存，供後續處理使用，這時可以建立一個 struct 包含以上資訊，並實作本 trait ：
/// ```rust
/// #[derive(Debug)]
/// struct ExampleModbusTarget(serde_json::Value)
///
/// impl Target for ExampleModbusTarget {}
/// ```
pub trait Target: Debug + Send + Sync + DowncastSync + DynClone + 'static {}
impl_downcast!(Target);
clone_trait_object!(Target);

/// 設備狀態請求
///
/// 實作本 trait 的 struct/enum 代表其定義了外部界面（API, RPC, MQTT Subscription etc.）需要哪些資料才能和設備連線請求狀態
///
/// 實作本 trait 的 struct/enum 會在 [`Connection::init_targets()`] function 中從 [`Connection::Target`] 定義的 struct/enum 轉換而來，實作 [`Connection`] trait 時，需要在 [`Connection::Request`] type ailas 指定一種有實作本 trait 的 struct/enum
///
/// 後續可在 [`Connection::preprocess()`] function 中進行預處理，在 [`Connection::request_process()`] 作為執行參數，並在 [`Connection::postprocess()`] 中作為後處理時可用的參數。
///
/// # 實作要求
///
/// 實作本 trait 的 struct/enum 必須同時實作 [`Debug`], [`Send`] 和 [`Sync`] 三個 trait 、持有 `'static` lifetime 且維持 [dyn-compatible](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
///
/// - [`Debug`]：可以輸出偵錯用資訊
/// - [`Send`]：可以被傳送至其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - [`Sync`]：可以被分享給其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - `'static` lifetime：標記引用需要在程式運行期間均有效
/// - dyn-compatible：要求實作後依然保持可以利用[動態分派 (dynamic dispatch)](https://zh.wikipedia.org/zh-tw/动态分派)
///
/// # 範例
/// Modbus RTU 存取某個 Register 需要定義 Modbus ID 、指令碼、資料地址與資料長度，並在後處理時根據預先定義的資料類型，進行資料型別轉換，這時可以建立一個 struct 包含以上資訊，並實作本 trait ：
/// ```rust
/// #[derive(Debug)]
/// struct ExampleModbusRequest {
///     id: u8,
///     function_code: u8,
///     address: u16,
///     length: u16,
///     data_type: DataType,
/// }
///
/// impl DeviceStateRequest for ExampleModbusRequest {}
/// ```
pub trait DeviceStateRequest: Debug + Send + Sync + DowncastSync + DynClone + 'static {}
impl_downcast!(DeviceStateRequest);
clone_trait_object!(DeviceStateRequest);

/// 設備狀態回覆
///
/// 實作本 trait 的 struct/enum 代表其定義了外部界面（API, RPC, MQTT Subscription etc.）可以獲得哪些資料作為回覆值
///
/// 實作本 trait 的 struct/enum 會在 [`Connection::request_process()`] function 而產生，實作 [`Connection`] trait 時，需要在 [`Connection::Response`] type ailas 指定一種有實作本 trait 的 struct/enum
///
/// 後續可在 [`Connection::postprocess()`] 中，利用 [`Connection::Request`] 所定義的 struct/enum 獲取相關參數進行後處理。
///
/// # 實作要求
///
/// 實作本 trait 的 struct/enum 必須同時實作 [`Debug`], [`Send`] 和 [`Sync`] 三個 trait 、持有 `'static` lifetime 且維持 [dyn-compatible](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
///
/// - [`Debug`]：可以輸出偵錯用資訊
/// - [`Send`]：可以被傳送至其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - [`Sync`]：可以被分享給其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - `'static` lifetime：標記引用需要在程式運行期間均有效
/// - dyn-compatible：要求實作後依然保持可以利用[動態分派 (dynamic dispatch)](https://zh.wikipedia.org/zh-tw/动态分派)
///
/// # 範例
/// Modbus RTU 存取某個 Register 後會得到多個 Modbus Word (一個 Word 為兩個 byte，可以利用 [`u16`] 儲存) 作為回覆值，基於實用性考慮，預留一個欄位供後處理進行資料型別轉換後，結果的存放位置，這時可以建立一個 struct 包含以上資訊，並實作本 trait ：
/// ```rust
/// #[derive(Debug)]
/// struct ExampleModbusResponse {
///     raw_words: Vec<u16>,
///     processed_value: Option<serde_json::Value>,
/// }
///
/// impl DeviceStateResponse for ExampleModbusResponse {
///     fn to_value(&self) -> serde_json::Value {
///         Value::from(self.processed_value)
///     }
/// }
/// ```
pub trait DeviceStateResponse: Debug + Send + Sync + DowncastSync + DynClone + 'static {
    /// 轉換為 [`serde_json`](https://crates.io/crates/serde_json) 的 [`serde_json::Value`]
    ///
    /// 本 method 用於方便後續程式邏輯將回傳值透過網路進行傳輸。
    fn to_value(&self) -> Value;
}
impl_downcast!(DeviceStateResponse);
clone_trait_object!(DeviceStateResponse);

/// 設備連線定義
///
/// 實作本 trait 的 struct/enum 代表著一種硬體，主程式在編譯期會自動掃描有實作本 trait
/// 的所有 struct/enum ，並利用 code generation 生成對外部請求分類、內部自動更新及通知狀態變更的邏輯
///
/// 詳細運作方式請見 crate doc
///
/// # 實作要求
///
/// 實作本 trait 的 struct/enum 必須同時實作 [`Sized`] 和 [`Send`] 兩個 trait 並持有 `'static` lifetime
///
/// - [`Sized`]：要求所有物件在編譯期時就知道其 stack 大小（編譯器會自動判斷是否適用，不需要手動實作）
/// - [`Send`]：可以被傳送至其他線程（編譯器會自動判斷是否適用，不需要手動實作）
/// - `'static` lifetime：標記引用需要在程式運行期間均有效
#[expect(async_fn_in_trait, unused_variables)]
pub trait Connection: Sized + Send + 'static {
    /// 設備型態名稱列表
    ///
    /// 設備在初始化時，會將設定檔中的設備型態與此處定義的設備名稱進行檢查，來判斷是否需要利用這個定義去初始化設備連線
    ///
    /// 請注意，一個硬體設備連線僅能配對至一個設備連線定義，且硬體連線中的所有設備，其型態均需包含在此列表中
    ///
    /// # 範例
    ///
    /// 此處定義了「A」與「B」兩種設備型態：
    ///
    /// ```rust
    /// const NAMES: &[&str] = &["A", "B"];
    /// ```
    ///
    /// - 連線中的所有設備僅有「A」設備型態 ✅
    /// - 連線中的設備有「A」與「B」設備型態夾雜在一起 ✅
    /// - 連線中的設備有「A」、「B」與「C」設備型態夾雜在一起 ❌
    /// - 連線中的設備有「A」與「C」設備型態夾雜在一起 ❌
    /// - 連線定義和其他連線定義衝突 ⚠️ 👉 沒有定義其行為，如果編譯期沒有噴錯，那運行期就會變成先搶先贏，所以請不要這麼做
    const NAMES: &[&str];

    /// 定義連線參數的型別
    ///
    /// 需為實作 [`ConnectionConfig`] trait 的 struct/enum
    type Config: ConnectionConfig;

    /// 定義點位的型別
    ///
    /// 需為實作 [`Target`] trait 的 struct/enum
    type Target: Target;

    /// 設備狀態請求型別
    ///
    /// 需為實作 [`DeviceStateRequest`] trait 的 struct/enum
    type Request: DeviceStateRequest;

    /// 設備狀態回覆型別
    ///
    /// 需為實作 [`DeviceStateResponse`] trait 的 struct/enum
    type Response: DeviceStateResponse;

    /// 定義將狀態回覆給外部服務的型別
    type Result;

    /// 初始化設備連線
    ///
    /// 主程式會在設備初始化時自動調用此 function ，實作者需要在此處實作連線到設備的邏輯
    ///
    /// 雖然被定義為 `async` ，但主程式會等待所有設備初始化結果回傳後，才會開始進行自動更新及接受外部請求
    ///
    /// # 參數
    /// - `config`：連線參數的引用（指派到 [`Self::Config`] 的型別）
    ///
    /// # 回傳值
    /// 「連線產品」、「最大重試次數」（非必需）及「執行間隔」，可回傳錯誤
    async fn init(
        config: &Self::Config,
    ) -> Result<ConnectionArtifact<Self>, Box<dyn std::error::Error>>;

    /// 初始化點位
    ///
    /// 主程式會在設備初始化後自動調用此 function ，實作者需要在此處將設定檔中的點位轉換成和 [`Self::Request`] 相同型別的請求與和 [`Self::Result`] 相同型別的回覆值，並分類是否自動更新
    ///
    /// 此 function 並不是 async function ，請不要在此處執行需要長時間等待的邏輯
    ///
    /// # 參數
    /// - `connection_statistics`：連線統計數據，如需記錄本設備的連線狀況，請修改此資料結構
    /// - `targets`：未處理的點位（指派到 [`Self::Target`] 的型別）
    ///
    /// # 回傳值
    ///
    /// 兩個不同類型點位的陣列，不可回傳錯誤
    ///
    /// 如有無法正常處理的點位，請在顯示完錯誤訊息後跳過，沒有傳入的點位會直接被主程式忽略
    fn init_targets(
        &mut self,
        connection_statistics: &mut ConnectionStats,
        targets: Vec<Self::Target>,
    ) -> ConnectionTargets<Self::Request, Self::Result>;

    /// 預處理（非必需）
    ///
    /// 主程式會在接收到外來服務的請求後，解析確認請求合法後，於正式執行前調用此 function ，實作者可以在這個 function 中對請求先進行一些更動
    ///
    /// 此 function 並不是 async function ，請不要在此處執行需要長時間等待的邏輯
    ///
    /// # 參數
    /// - `request`：傳入的請求
    /// - `new_status`：將被更新的新狀態
    ///
    /// # 回傳值
    /// 新的與 [`Self::Request`] 相同型別的請求，可回傳錯誤
    #[expect(clippy::missing_errors_doc)]
    fn preprocess(
        &self,
        request: Self::Request,
        new_status: Option<String>,
    ) -> Result<Self::Request, Box<dyn std::error::Error>> {
        Ok(request)
    }

    /// 處理請求
    ///
    /// 主程式在準備好請求後，會在指定的間隔調用此 function ，實作者需要在這個 function 中定義如何與設備進行資料交換
    ///
    /// 主程式會先處理由外部服務傳入的請求，如當前沒有外部服務請求，將會輪詢自動更新點位
    ///
    /// 此 function 是 async function ，其間隔時間與逾時設定和 [`ConnectionArtifact::update_interval`] 的設定值相同
    ///
    /// 如遇到作業系統時鐘校時、硬體性能不足等其他不可抗力因素導致無法在間隔時間內處理到請求，主程式將會直接跳過指令不執行
    ///
    /// # 參數
    /// - `request`：傳入的請求
    ///
    /// # 回傳值
    /// 與 [`Self::Response`] 相同型別的回覆，可回傳錯誤
    /// union 中的布林代表「是否等待間隔」，如傳入 `false` 主程式會跳過等待間隔，直接執行下一個操作
    async fn request_process(
        &mut self,
        request: Self::Request,
    ) -> Result<(Self::Response, bool), Box<dyn std::error::Error>>;

    /// 後處理（非必需）
    ///
    /// 主程式會在接收到來自設備的狀態後，於儲存前調用此 function
    ///
    /// 此 function 並不是 async function ，請不要在此處執行需要長時間等待的邏輯
    ///
    /// # 參數
    /// - `request`：傳入的請求
    /// - `response`：設備的回覆值
    ///
    /// # 回傳值
    /// 新的與 [`Self::Response`] 相同型別的回覆，可回傳錯誤
    #[expect(clippy::missing_errors_doc)]
    #[expect(unused_variables)]
    fn postprocess(
        &self,
        request: Self::Request,
        response: Self::Response,
    ) -> Result<Self::Response, Box<dyn std::error::Error>> {
        Ok(response)
    }

    /// 重新連線
    ///
    /// 主程式會在失敗次數大於 [`ConnectionArtifact::max_retry_count`] 後調用此 function
    ///
    /// # 回傳值
    /// 無，可回傳錯誤
    async fn reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// 利用新傳入的設定檔重新連線
    ///
    /// 主程式會在接收到新設定檔，調用此 function 更新連線
    ///
    /// # 回傳值
    /// 無，可回傳錯誤
    async fn update_config(
        &mut self,
        new_config: &Self::Config,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// 設備連線產品
///
/// 本 struct 於 [`Connection::init()`] 執行成功時產生，用於包裝成功建立的設備連線、最高重試次數及更新間隔
///
/// 泛型 `T` 為 [`Connection`] trait 的實作者
pub struct ConnectionArtifact<T: Connection> {
    /// 成功建立的設備連線
    pub artifact: T,
    /// 最高重試次數
    ///
    /// 程式會在失敗次數累加到等於此處設定的數值後，嘗試利用 [`Connection::reconnect()`] function 重新建立連線，如未定義本數值，則不會自動重新建立連線
    pub max_retry_count: Option<u32>,
    /// 更新間隔
    ///
    /// 程式會依據此處設定的數字，以毫秒為單位作為間隔去處理請求
    pub update_interval: u64,
    /// 逾時
    ///
    /// 程式會依據此處設定的數字，以毫秒為單位，當操作所需時間大於此處設定值時終止操作
    pub timeout: u64,
    /// 連線統計數據
    pub statistics: ConnectionStats,
}

/// 設備連線所屬的點位
///
/// 本 struct 於 [`Connection::init_targets()`] 作為回傳值，用於存放該連線所屬的點位
///
/// 泛型 `REQ` 為 [`DeviceStateRequest`] trait 的實作者，泛型 `RES` 可根據需要回傳的資料格式進行自訂
#[derive(Debug, Clone)]
pub struct ConnectionTargets<REQ, RES>(pub Vec<InitedTarget<REQ, RES>>)
where
    REQ: DeviceStateRequest;

/// 已初始化的點位
///
/// 本 struct 於 [`Connection::init_targets()`] 執行後產生的 [`ConnectionTargets<REQ, RES>`] struct
/// 中作為元素存在，程式會存取本 struct 中定義的內容運作
///
/// 泛型 `REQ` 為 [`DeviceStateRequest`] trait 的實作者，泛型 `RES` 可根據需要回傳的資料格式進行自訂
#[derive(Debug, Clone)]
pub struct InitedTarget<REQ, RES>
where
    REQ: DeviceStateRequest,
{
    /// 點位名稱
    pub name: String,
    /// 存取點位時，需要使用到的請求
    pub request: REQ,
    /// 向外部服務回傳資料時，所需要的資訊
    ///
    /// 當程式處理完請求後，會依程式定義將結果儲存至本資料結構中
    pub result: RES,
    /// 點位初始狀態
    ///
    /// 當點位尚未取得最新數值時，預設顯示的狀態
    pub default_status: Option<Value>,
    /// 是否要自動更新
    pub auto_refresh: bool,
    /// 點位統計數據
    ///
    /// 非必填，如果需要記錄設備連線狀態，請在 [`Connection::init_targets()`] 的 `connection_statistics` 參數中初始化新的 [`TargetStats`] ，並利用 [`Arc::clone()`] 方法複製一份指針至此
    pub statistics: Option<Arc<TargetStats>>,
}

/// 連線統計數據
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub port_target: String,
    pub port_note: Option<String>,
    pub targets: HashMap<TargetAddressNumber, Arc<TargetStats>>,
}

impl ConnectionStats {
    /// 取得點位統計數據
    #[must_use]
    pub fn get_target(&self, address_number: &Option<String>) -> Option<&Arc<TargetStats>> {
        self.targets.get(address_number)
    }

    /// 取得加總/平均統計數據
    ///
    /// # 回傳
    /// 加總/平均統計數據，參見 [`Statistics`]
    #[must_use]
    pub fn get_all_stats(&self) -> Statistics {
        self.targets
            .values()
            .fold(Statistics::default(), |accumulator, next_target| {
                accumulator.failed_poll_count.fetch_add(
                    next_target
                        .0
                        .failed_poll_count
                        .load(std::sync::atomic::Ordering::Relaxed),
                    std::sync::atomic::Ordering::Relaxed,
                );

                accumulator.total_polling_count.fetch_add(
                    next_target
                        .0
                        .total_polling_count
                        .load(std::sync::atomic::Ordering::Relaxed),
                    std::sync::atomic::Ordering::Relaxed,
                );

                let _ = accumulator.average_response_ms.fetch_update(
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                    |average_response_ms| {
                        let current_total_polling_count = accumulator
                            .total_polling_count
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let current_failed_polling_count = accumulator
                            .failed_poll_count
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let next_total_polling_count = next_target
                            .0
                            .total_polling_count
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let next_failed_polling_count = next_target
                            .0
                            .failed_poll_count
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let next_average_response_ms = next_target
                            .0
                            .average_response_ms
                            .load(std::sync::atomic::Ordering::Relaxed);

                        let current_success_count =
                            current_total_polling_count - current_failed_polling_count;
                        let next_success_count =
                            next_total_polling_count - next_failed_polling_count;

                        Some(
                            ((average_response_ms * current_success_count)
                                + (next_average_response_ms * next_success_count))
                                / (current_success_count + next_success_count),
                        )
                    },
                );

                accumulator
            })
    }
}

/// 連線統計數據設備編號
///
/// 請填入點位物理連線中，鏈狀結構用於區分實體設備的設備編號，如於 `COM1` 的 Modbus
/// 設備，請填入該設備的 Modbus device ID
pub type TargetAddressNumber = Option<String>;

/// 點位統計數據
#[derive(Debug, Default)]
pub struct TargetStats(Statistics);

impl TargetStats {
    /// 記錄請求成功
    ///
    /// # 參數
    /// - `response_ms`: 本次請求所花費的毫秒數
    pub fn record_success(&self, response_ms: i64) {
        let orig_polling_count = self
            .0
            .total_polling_count
            .fetch_add(1, std::sync::atomic::Ordering::Acquire);

        let orig_failed_poll_count = self
            .0
            .failed_poll_count
            .load(std::sync::atomic::Ordering::Acquire);

        let orig_response_ms = self
            .0
            .average_response_ms
            .load(std::sync::atomic::Ordering::Acquire);

        let new_response_ms = ((orig_response_ms * (orig_polling_count - orig_failed_poll_count))
            + response_ms)
            / (orig_polling_count + 1);

        self.0
            .average_response_ms
            .store(new_response_ms, std::sync::atomic::Ordering::Release);
    }

    /// 記錄請求失敗
    pub fn record_failure(&self) {
        self.0
            .total_polling_count
            .fetch_add(1, std::sync::atomic::Ordering::Acquire);

        self.0
            .failed_poll_count
            .fetch_add(1, std::sync::atomic::Ordering::Acquire);
    }

    pub fn get_latest_value(&self) -> (i64, i64, i64) {
        (
            self.0
                .failed_poll_count
                .load(std::sync::atomic::Ordering::Acquire),
            self.0
                .total_polling_count
                .load(std::sync::atomic::Ordering::Acquire),
            self.0
                .average_response_ms
                .load(std::sync::atomic::Ordering::Acquire),
        )
    }

    pub fn clear(&self) {
        self.0
            .failed_poll_count
            .store(i64::default(), std::sync::atomic::Ordering::Release);
        self.0
            .total_polling_count
            .store(i64::default(), std::sync::atomic::Ordering::Release);
        self.0
            .average_response_ms
            .store(i64::default(), std::sync::atomic::Ordering::Release);
    }
}

/// 統計數據
///
/// 注意，本物件雖然實作 [`Send`] 和 [`Sync`] trait ，但因為 [Atomic 的特殊性](https://doc.rust-lang.org/stable/std/sync/atomic/index.html)，需要跨線程存取時，請利用 [`Arc`] 智慧指針
#[derive(Debug, Default)]
pub struct Statistics {
    /// 失敗的輪詢次數
    failed_poll_count: AtomicI64,
    /// 總輪詢次數
    total_polling_count: AtomicI64,
    /// 平均回覆毫秒數
    average_response_ms: AtomicI64,
}
