//! Bean-style `getXxx` / `isXxx` fallback for hash models.
//!
//! Java FreeMarker wraps POJOs via BeansWrapper so `${sceneEnum.getName()}` works.
//! Rust bindings usually dump JSON maps with property keys (`name`, `basePackage`).
//! When a key like `getName` is missing, resolve property `name` and return a
//! zero-arg method model that yields that value — enough for codegen FTLs that
//! still call Java getters on JSON-backed hashes.

use crate::core::Environment;
use crate::error::Result;
use crate::template::{TModel, TemplateMethodModelEx};

/// JavaBeans Introspector.decapitalize for getter suffix → property name.
pub(crate) fn decapitalize_prop(rest: &str) -> Option<String> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    // If first two chars are uppercase, leave as-is (URL → URL, not uRL).
    if chars.clone().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return Some(rest.to_string());
    }
    let mut out = String::with_capacity(rest.len());
    out.extend(first.to_lowercase());
    out.push_str(chars.as_str());
    Some(out)
}

/// `getName` → `name`, `isPrimaryKey` → `primaryKey`; else None.
pub(crate) fn bean_property_from_accessor(key: &str) -> Option<String> {
    if let Some(rest) = key.strip_prefix("get") {
        return decapitalize_prop(rest);
    }
    if let Some(rest) = key.strip_prefix("is") {
        return decapitalize_prop(rest);
    }
    None
}

/// Zero-arg method that always returns a fixed model (Java getter on a map value).
pub(crate) struct ConstantReturnMethod(pub TModel);

impl TemplateMethodModelEx for ConstantReturnMethod {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        if !args.is_empty() {
            // Real bean getters take no args; ignore extras for template resilience.
        }
        Ok(self.0.clone())
    }
}

/// If `key` matches getXxx/isXxx, wrap the property (or nothing) as a zero-arg method.
/// Missing properties still yield a method returning nothing — matches Java bean getters
/// that exist but return null (`c.getDictDatas()??` must not fail as "not a method").
pub(crate) fn bean_getter_fallback(
    get_prop: impl FnOnce(&str) -> Option<TModel>,
    key: &str,
) -> Option<TModel> {
    let prop = bean_property_from_accessor(key)?;
    let v = get_prop(&prop).unwrap_or_else(TModel::nothing);
    Some(TModel::from_method(ConstantReturnMethod(v)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decapitalize_name() {
        assert_eq!(decapitalize_prop("Name").as_deref(), Some("name"));
        assert_eq!(decapitalize_prop("URL").as_deref(), Some("URL"));
        assert_eq!(decapitalize_prop("BasePackage").as_deref(), Some("basePackage"));
    }

    #[test]
    fn accessor_keys() {
        assert_eq!(
            bean_property_from_accessor("getName").as_deref(),
            Some("name")
        );
        assert_eq!(
            bean_property_from_accessor("isPrimaryKey").as_deref(),
            Some("primaryKey")
        );
        assert_eq!(bean_property_from_accessor("name"), None);
    }
}
