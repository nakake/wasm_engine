use std::any::Any;

/// 全コンポーネントが実装すべきマーカートレイト
/// - `'static`: コンポーネントは参照を持たない
/// - `Send + Sync`: 将来のマルチスレッド対応
pub trait Component: 'static + Send + Sync {}

/// 型消去されたストレージからの復元用トレイト
pub trait AsAny: Any {
    /// 不変参照としてAnyにキャスト
    fn as_any(&self) -> &dyn Any;
    /// 可変参照としてAnyにキャスト
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestComponent {
        value: i32,
    }

    impl Component for TestComponent {}

    #[test]
    fn test_as_any_downcast() {
        let comp = TestComponent { value: 42 };
        let any_ref: &dyn Any = comp.as_any();
        let downcasted = any_ref.downcast_ref::<TestComponent>().unwrap();
        assert_eq!(downcasted.value, 42);
    }

    #[test]
    fn test_as_any_mut_downcast() {
        let mut comp = TestComponent { value: 10 };
        {
            let any_mut: &mut dyn Any = comp.as_any_mut();
            let downcasted = any_mut.downcast_mut::<TestComponent>().unwrap();
            downcasted.value = 20;
        }
        assert_eq!(comp.value, 20);
    }
}
