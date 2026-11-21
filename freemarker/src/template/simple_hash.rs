//! 简单哈希 —— 对应 Java `freemarker.template.SimpleHash`
//! （?keys/?values 顺序 = 插入序；Java SimpleHash 内部为 LinkedHashMap，
//! v1 用 indexmap 保持插入序）
//!
//! 内部表用 FNV 哈希（`utility::FnvBuildHasher`）：哈希成员访问是渲染热路径
//! （`${big.key_0}` 每次点访问一次 get），FNV 对短 ASCII 键比 SipHash 快 3-5 倍；
//! 插入序语义不受哈希器影响（indexmap 的序由内部向量维持）。
//!
//! Bean-style：键 `getName` / `isFoo` 缺失时，回退到属性 `name` / `foo` 并返回
//! 零参方法模型（见 `bean_getter`）——对齐 BeansWrapper 上 JSON/Map 绑定的
//! `${obj.getName()}` 用法。

use crate::error::Result;
use crate::template::bean_getter::bean_getter_fallback;
use crate::template::utility::FnvBuildHasher;
use crate::template::TModel;
use crate::template::{TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;

pub struct SimpleHash(pub IndexMap<String, TModel, FnvBuildHasher>);
impl TemplateHashModel for SimpleHash {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        if let Some(v) = self.0.get(key) {
            return Ok(Some(v.clone()));
        }
        Ok(bean_getter_fallback(|prop| self.0.get(prop).cloned(), key))
    }
    fn is_empty(&self) -> Result<bool> {
        Ok(self.0.is_empty())
    }
}
impl TemplateHashModelEx for SimpleHash {
    fn size(&self) -> Result<usize> {
        Ok(self.0.len())
    }
    fn keys(&self) -> Result<Vec<String>> {
        Ok(self.0.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use std::sync::Arc;

    fn render(src: &str, root: DynValue) -> String {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl").unwrap();
        let root_model = SimpleObjectWrapper
            .wrap(&root)
            .unwrap()
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn get_name_method_on_hash() {
        let root = DynValue::Map(vec![(
            "sceneEnum".into(),
            DynValue::Map(vec![
                ("name".into(), DynValue::Str("ADMIN".into())),
                ("basePackage".into(), DynValue::Str("admin".into())),
            ]),
        )]);
        assert_eq!(
            render(
                "${sceneEnum.getName()}-${sceneEnum.basePackage}",
                root
            ),
            "ADMIN-admin"
        );
    }

    #[test]
    fn missing_getter_returns_null_method() {
        let root = DynValue::Map(vec![(
            "c".into(),
            DynValue::Map(vec![("columnName".into(), DynValue::Str("id".into()))]),
        )]);
        assert_eq!(
            render("<#if c.getDictDatas()??>yes<#else>no</#if>", root),
            "no"
        );
    }
}
