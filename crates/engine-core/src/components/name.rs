use crate::ecs::Component;

/// Nameコンポーネント
/// Entityの表示名を表す
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name {
    value: String,
}

impl Name {
    /// 新しいNameコンポーネントを作成
    pub fn new(name: impl Into<String>) -> Self {
        Self { value: name.into() }
    }

    /// 名前を文字列スライスとして取得
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Default for Name {
    fn default() -> Self {
        Self::new("Entity")
    }
}

impl Component for Name {}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let name = Name::new("Player");
        assert_eq!(name.as_str(), "Player");
    }

    #[test]
    fn test_from_string() {
        let name = Name::new(String::from("Enemy"));
        assert_eq!(name.as_str(), "Enemy");
    }

    #[test]
    fn test_default() {
        let name = Name::default();
        assert_eq!(name.as_str(), "Entity");
    }

    #[test]
    fn test_display() {
        let name = Name::new("Camera");
        assert_eq!(format!("{}", name), "Camera");
    }

    #[test]
    fn test_clone() {
        let name1 = Name::new("Original");
        let name2 = name1.clone();
        assert_eq!(name1, name2);
    }
}
