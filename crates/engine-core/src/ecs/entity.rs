use std::num::NonZeroU32;

/// Entity識別子（世代番号付き）
/// - index: Entity配列のインデックス
/// - generation: 再利用時の世代番号（削除済みEntityとの区別用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    index: u32,
    generation: NonZeroU32,
}

impl EntityId {
    /// 新しいEntityIdを作成
    /// generationは1以上である必要がある（0は無効値として予約）
    pub fn new(index: u32, generation: u32) -> Self {
        Self {
            index,
            generation: NonZeroU32::new(generation).expect("generation must be >= 1"),
        }
    }

    /// インデックスを取得
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// 世代番号を取得
    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation.get()
    }

    /// JS用の単純なID変換（上位12bit: generation, 下位20bit: index）
    /// 最大約100万Entity、世代4096まで対応
    #[inline]
    pub fn to_u32(&self) -> u32 {
        let generation_bits = (self.generation.get() & 0xFFF) << 20;
        let index_bits = self.index & 0xFFFFF;
        generation_bits | index_bits
    }

    /// JS用IDからEntityIdを復元
    #[inline]
    pub fn from_u32(id: u32) -> Self {
        let generation = (id >> 20) & 0xFFF;
        let index = id & 0xFFFFF;
        Self::new(index, generation.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_accessors() {
        let entity = EntityId::new(42, 1);
        assert_eq!(entity.index(), 42);
        assert_eq!(entity.generation(), 1);
    }

    #[test]
    fn test_equality() {
        let a = EntityId::new(1, 1);
        let b = EntityId::new(1, 1);
        let c = EntityId::new(1, 2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_to_u32_roundtrip() {
        let original = EntityId::new(12345, 7);
        let packed = original.to_u32();
        let restored = EntityId::from_u32(packed);
        assert_eq!(original.index(), restored.index());
        assert_eq!(original.generation(), restored.generation());
    }

    #[test]
    fn test_to_u32_max_values() {
        // 最大index (20bit = 1048575)
        let max_index = EntityId::new(0xFFFFF, 1);
        assert_eq!(EntityId::from_u32(max_index.to_u32()).index(), 0xFFFFF);

        // 最大generation (12bit = 4095)
        let max_gen = EntityId::new(0, 4095);
        assert_eq!(EntityId::from_u32(max_gen.to_u32()).generation(), 4095);
    }

    #[test]
    #[should_panic(expected = "generation must be >= 1")]
    fn test_generation_zero_panics() {
        EntityId::new(0, 0);
    }
}
