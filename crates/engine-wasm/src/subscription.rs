//! クエリ購読管理モジュール
//!
//! クエリ結果の変更を監視し、コールバックを呼び出す

use js_sys::Function;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use engine_core::{QueryDescriptor, QueryResult};

/// クエリ購読情報
pub struct QuerySubscription {
    pub query: QueryDescriptor,
    pub callback: Function,
    pub last_result_hash: u64,
}

/// 購読マネージャー
pub struct QuerySubscriptionManager {
    subscriptions: HashMap<u32, QuerySubscription>,
    next_id: u32,
}

impl QuerySubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn subscribe(&mut self, query: QueryDescriptor, callback: Function) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.subscriptions.insert(
            id,
            QuerySubscription {
                query,
                callback,
                last_result_hash: 0,
            },
        );

        id
    }

    pub fn unsubscribe(&mut self, id: u32) -> bool {
        self.subscriptions.remove(&id).is_some()
    }

    /// 全購読の可変参照を取得
    #[allow(dead_code)]
    pub fn subscriptions_mut(&mut self) -> &mut HashMap<u32, QuerySubscription> {
        &mut self.subscriptions
    }

    /// 全購読のキーを取得
    pub fn subscription_ids(&self) -> Vec<u32> {
        self.subscriptions.keys().copied().collect()
    }

    /// 特定の購読を取得
    pub fn get_mut(&mut self, id: u32) -> Option<&mut QuerySubscription> {
        self.subscriptions.get_mut(&id)
    }
}

impl Default for QuerySubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// クエリ結果のハッシュを計算
pub fn calculate_hash(result: &QueryResult) -> u64 {
    let json = serde_json::to_string(result).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    hasher.finish()
}
