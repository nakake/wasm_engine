//! Query system for ECS
//!
//! Provides SQL-like query capabilities for entities and components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// CompareOp - 比較演算子
// ============================================================================

/// 比較演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    /// 等しい (==)
    #[serde(rename = "==")]
    Eq,

    /// 等しくない (!=)
    #[serde(rename = "!=")]
    Ne,

    /// より小さい (<)
    #[serde(rename = "<")]
    Lt,

    /// 以下 (<=)
    #[serde(rename = "<=")]
    Le,

    /// より大きい (>)
    #[serde(rename = ">")]
    Gt,

    /// 以上 (>=)
    #[serde(rename = ">=")]
    Ge,
}

impl CompareOp {
    /// 2つの値を比較する
    pub fn compare<T: PartialOrd>(&self, left: &T, right: &T) -> bool {
        match self {
            CompareOp::Eq => left == right,
            CompareOp::Ne => left != right,
            CompareOp::Lt => left < right,
            CompareOp::Le => left <= right,
            CompareOp::Gt => left > right,
            CompareOp::Ge => left >= right,
        }
    }

    /// FilterValue同士の比較
    pub fn compare_values(&self, left: &FilterValue, right: &FilterValue) -> bool {
        match (left, right) {
            (FilterValue::Number(l), FilterValue::Number(r)) => self.compare(l, r),
            (FilterValue::String(l), FilterValue::String(r)) => self.compare(l, r),
            (FilterValue::Bool(l), FilterValue::Bool(r)) => self.compare(l, r),
            (FilterValue::Null, FilterValue::Null) => matches!(self, CompareOp::Eq),
            _ => false, // 型が異なる場合は常にfalse
        }
    }
}

// ============================================================================
// FilterValue - フィルター値
// ============================================================================

/// フィルターで使用可能な値の型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

impl FilterValue {
    /// f64として取得
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            FilterValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// 文字列として取得
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FilterValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// boolとして取得
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FilterValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Nullかどうか
    pub fn is_null(&self) -> bool {
        matches!(self, FilterValue::Null)
    }

    /// serde_json::ValueからFilterValueを作成
    pub fn from_json(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Number(n) => {
                FilterValue::Number(n.as_f64().unwrap_or(0.0))
            }
            serde_json::Value::String(s) => FilterValue::String(s),
            serde_json::Value::Bool(b) => FilterValue::Bool(b),
            serde_json::Value::Null => FilterValue::Null,
            // オブジェクトや配列はサポートしない（Nullとして扱う）
            _ => FilterValue::Null,
        }
    }

    /// FilterValueをserde_json::Valueに変換
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            FilterValue::Number(n) => serde_json::json!(*n),
            FilterValue::String(s) => serde_json::json!(s),
            FilterValue::Bool(b) => serde_json::json!(*b),
            FilterValue::Null => serde_json::Value::Null,
        }
    }
}

impl PartialEq for FilterValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FilterValue::Number(a), FilterValue::Number(b)) => a == b,
            (FilterValue::String(a), FilterValue::String(b)) => a == b,
            (FilterValue::Bool(a), FilterValue::Bool(b)) => a == b,
            (FilterValue::Null, FilterValue::Null) => true,
            _ => false,
        }
    }
}

// ============================================================================
// FilterExpr - フィルター式
// ============================================================================

/// フィルター式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterExpr {
    /// フィールド名 (e.g., "health", "position.x")
    pub field: String,

    /// 比較演算子
    pub op: CompareOp,

    /// 比較値
    pub value: FilterValue,
}

impl FilterExpr {
    /// 新しいフィルター式を作成
    pub fn new(field: impl Into<String>, op: CompareOp, value: FilterValue) -> Self {
        Self {
            field: field.into(),
            op,
            value,
        }
    }

    /// 等価フィルター (field == value)
    pub fn eq(field: impl Into<String>, value: FilterValue) -> Self {
        Self::new(field, CompareOp::Eq, value)
    }

    /// 不等価フィルター (field != value)
    pub fn ne(field: impl Into<String>, value: FilterValue) -> Self {
        Self::new(field, CompareOp::Ne, value)
    }

    /// より小さいフィルター (field < value)
    pub fn lt(field: impl Into<String>, value: FilterValue) -> Self {
        Self::new(field, CompareOp::Lt, value)
    }

    /// 以下フィルター (field <= value)
    pub fn le(field: impl Into<String>, value: FilterValue) -> Self {
        Self::new(field, CompareOp::Le, value)
    }

    /// より大きいフィルター (field > value)
    pub fn gt(field: impl Into<String>, value: FilterValue) -> Self {
        Self::new(field, CompareOp::Gt, value)
    }

    /// 以上フィルター (field >= value)
    pub fn ge(field: impl Into<String>, value: FilterValue) -> Self {
        Self::new(field, CompareOp::Ge, value)
    }
}

// ============================================================================
// ComponentFilter - コンポーネント存在フィルター
// ============================================================================

/// コンポーネント存在フィルター
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentFilter {
    /// コンポーネントを持つ
    Has(String),
    /// コンポーネントを持たない
    Not(String),
}

// ============================================================================
// SortDirection & OrderBy - ソート
// ============================================================================

/// ソート方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        SortDirection::Asc
    }
}

/// ソート条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    /// ソート対象フィールド
    pub field: String,
    /// ソート方向
    pub direction: SortDirection,
}

impl OrderBy {
    /// 昇順ソート
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Asc,
        }
    }

    /// 降順ソート
    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Desc,
        }
    }
}

// ============================================================================
// QueryDescriptor - クエリ定義
// ============================================================================

/// クエリ定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryDescriptor {
    /// 取得するフィールド名 (e.g., ["name", "position", "health"])
    pub select: Vec<String>,

    /// 必須コンポーネント (e.g., ["Enemy", "Health"])
    pub with_components: Vec<String>,

    /// 除外コンポーネント (e.g., ["Dead"])
    pub without_components: Vec<String>,

    /// フィルター条件
    pub filters: Vec<FilterExpr>,

    /// ソート条件
    pub order_by: Option<OrderBy>,

    /// 取得上限
    pub limit: Option<usize>,
}

impl QueryDescriptor {
    /// 新しいクエリを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// 取得フィールドを設定
    pub fn select(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.select = fields.into_iter().map(Into::into).collect();
        self
    }

    /// 必須コンポーネントを追加
    pub fn with(mut self, components: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.with_components = components.into_iter().map(Into::into).collect();
        self
    }

    /// 除外コンポーネントを追加
    pub fn without(mut self, components: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.without_components = components.into_iter().map(Into::into).collect();
        self
    }

    /// フィルター条件を追加
    pub fn filter(mut self, expr: FilterExpr) -> Self {
        self.filters.push(expr);
        self
    }

    /// ソート条件を設定
    pub fn order_by(mut self, order: OrderBy) -> Self {
        self.order_by = Some(order);
        self
    }

    /// 取得上限を設定
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// 必須コンポーネントを1つ追加
    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.with_components.push(component.into());
        self
    }

    /// 除外コンポーネントを1つ追加
    pub fn without_component(mut self, component: impl Into<String>) -> Self {
        self.without_components.push(component.into());
        self
    }
}

// ============================================================================
// QueryResult - クエリ結果
// ============================================================================

/// クエリ結果の1行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultRow {
    /// EntityId (u32形式)
    pub id: u32,

    /// 選択されたフィールドの値
    pub fields: HashMap<String, serde_json::Value>,
}

impl QueryResultRow {
    /// 新しい結果行を作成
    pub fn new(id: u32) -> Self {
        Self {
            id,
            fields: HashMap::new(),
        }
    }

    /// フィールドを追加
    pub fn with_field(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        self.fields.insert(name.into(), value);
        self
    }

    /// フィールドを設定
    pub fn set_field(&mut self, name: impl Into<String>, value: serde_json::Value) {
        self.fields.insert(name.into(), value);
    }

    /// フィールド値を取得
    pub fn get_field(&self, name: &str) -> Option<&serde_json::Value> {
        self.fields.get(name)
    }
}

/// クエリ実行結果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryResult {
    /// 結果の行
    pub rows: Vec<QueryResultRow>,

    /// limit適用前の総件数
    pub total_count: usize,
}

impl QueryResult {
    /// 空の結果を作成
    pub fn empty() -> Self {
        Self::default()
    }

    /// 結果行数
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// 結果が空かどうか
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// イテレータを取得
    pub fn iter(&self) -> impl Iterator<Item = &QueryResultRow> {
        self.rows.iter()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // CompareOp tests
    #[test]
    fn test_compare_op_numbers() {
        assert!(CompareOp::Eq.compare(&5, &5));
        assert!(!CompareOp::Eq.compare(&5, &6));
        assert!(CompareOp::Ne.compare(&5, &6));
        assert!(CompareOp::Lt.compare(&5, &6));
        assert!(CompareOp::Le.compare(&5, &5));
        assert!(CompareOp::Gt.compare(&6, &5));
        assert!(CompareOp::Ge.compare(&5, &5));
    }

    #[test]
    fn test_compare_op_serialize() {
        let op = CompareOp::Lt;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, r#""<""#);

        let op: CompareOp = serde_json::from_str(r#""<=""#).unwrap();
        assert_eq!(op, CompareOp::Le);
    }

    #[test]
    fn test_compare_values() {
        let a = FilterValue::Number(5.0);
        let b = FilterValue::Number(10.0);
        assert!(CompareOp::Lt.compare_values(&a, &b));
        assert!(!CompareOp::Gt.compare_values(&a, &b));

        let s1 = FilterValue::String("abc".to_string());
        let s2 = FilterValue::String("def".to_string());
        assert!(CompareOp::Lt.compare_values(&s1, &s2));
    }

    // FilterValue tests
    #[test]
    fn test_filter_value_accessors() {
        let num = FilterValue::Number(42.0);
        assert_eq!(num.as_f64(), Some(42.0));
        assert_eq!(num.as_str(), None);

        let s = FilterValue::String("hello".to_string());
        assert_eq!(s.as_str(), Some("hello"));
        assert_eq!(s.as_f64(), None);

        let b = FilterValue::Bool(true);
        assert_eq!(b.as_bool(), Some(true));

        let null = FilterValue::Null;
        assert!(null.is_null());
    }

    #[test]
    fn test_filter_value_serialize() {
        let num = FilterValue::Number(42.0);
        let json = serde_json::to_string(&num).unwrap();
        assert_eq!(json, "42.0");

        let s = FilterValue::String("hello".to_string());
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#""hello""#);
    }

    // FilterExpr tests
    #[test]
    fn test_filter_expr_builders() {
        let f = FilterExpr::lt("health", FilterValue::Number(50.0));
        assert_eq!(f.field, "health");
        assert_eq!(f.op, CompareOp::Lt);
        assert_eq!(f.value.as_f64(), Some(50.0));
    }

    // OrderBy tests
    #[test]
    fn test_order_by() {
        let asc = OrderBy::asc("name");
        assert_eq!(asc.field, "name");
        assert_eq!(asc.direction, SortDirection::Asc);

        let desc = OrderBy::desc("score");
        assert_eq!(desc.direction, SortDirection::Desc);
    }

    #[test]
    fn test_sort_direction_serialize() {
        let asc = SortDirection::Asc;
        let json = serde_json::to_string(&asc).unwrap();
        assert_eq!(json, r#""asc""#);
    }

    // QueryDescriptor tests
    #[test]
    fn test_query_descriptor_builder() {
        let query = QueryDescriptor::new()
            .select(["name", "health"])
            .with(["Enemy", "Health"])
            .without(["Dead"])
            .filter(FilterExpr::lt("health", FilterValue::Number(50.0)))
            .order_by(OrderBy::asc("health"))
            .limit(10);

        assert_eq!(query.select, vec!["name", "health"]);
        assert_eq!(query.with_components, vec!["Enemy", "Health"]);
        assert_eq!(query.without_components, vec!["Dead"]);
        assert_eq!(query.filters.len(), 1);
        assert!(query.order_by.is_some());
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_query_descriptor_serialize() {
        let query = QueryDescriptor::new()
            .with(["Transform"])
            .filter(FilterExpr::gt("position.x", FilterValue::Number(0.0)));

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("Transform"));
        assert!(json.contains("position.x"));
    }

    #[test]
    fn test_query_descriptor_deserialize() {
        let json = r#"{
            "select": ["name"],
            "with_components": ["Enemy"],
            "without_components": [],
            "filters": [],
            "order_by": null,
            "limit": 5
        }"#;

        let query: QueryDescriptor = serde_json::from_str(json).unwrap();
        assert_eq!(query.select, vec!["name"]);
        assert_eq!(query.with_components, vec!["Enemy"]);
        assert_eq!(query.limit, Some(5));
    }
}
