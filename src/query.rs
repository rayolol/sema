use crate::types::{
    ItemId, ResolvedEnum, ResolvedFunction, ResolvedImpl, ResolvedStruct, ResolvedTrait,
};

pub struct StructQuery<'a> {
    items: Box<dyn Iterator<Item = &'a ResolvedStruct> + 'a>,
}

pub trait SemaItem {
    fn attrs(&self) -> &[syn::Attribute];
    fn module_path(&self) -> String;
    fn id(&self) -> ItemId;
    fn file(&self) -> &std::path::Path;
    fn name(&self) -> String;
    fn is_public(&self) -> bool;
}

pub struct Query<'a, T: ?Sized> {
    items: Box<dyn Iterator<Item = &'a T> + 'a>,
}

impl<'a, T: SemaItem + ?Sized> Query<'a, T> {
    pub fn new(iter: impl Iterator<Item = &'a T> + 'a) -> Self {
        Query {
            items: Box::new(iter),
        }
    }
    pub fn filter(self, f: impl Fn(&T) -> bool + 'a) -> Self {
        Query::new(self.items.filter(move |item| f(item)))
    }

    pub fn with_attribute(self, name: &'a str) -> Self {
        self.filter(move |item| item.attrs().iter().any(|a| a.path().is_ident(name)))
    }

    // TODO add with derive later

    pub fn named(self, name: &'a str) -> Self {
        self.filter(move |item| item.name() == name)
    }

    pub fn public(self) -> Self {
        self.filter(|item| item.is_public())
    }

    pub fn in_module(self, path: &'a str) -> Self {
        self.filter(move |item| item.module_path().starts_with(path))
    }

    pub fn named_matching(self, f: impl Fn(&str) -> bool + 'a) -> Self {
        self.filter(move |item| f(&item.name()))
    }

    pub fn collect(self) -> Vec<&'a T> {
        self.items.collect()
    }

    pub fn first(self) -> Option<&'a T> {
        self.items.into_iter().next()
    }

    pub fn count(self) -> usize {
        self.items.count()
    }

    pub fn any(self) -> bool {
        self.items.into_iter().next().is_some()
    }

    pub fn with_impl(self, workspace: &crate::workspaces::Workspace) -> Self
    where
        T: 'a,
    {
        self.filter(move |_item| true) // placeholder - will enhance per type
    }
}

impl<'a, T: SemaItem + ?Sized> IntoIterator for Query<'a, T> {
    type Item = &'a T;
    type IntoIter = Box<dyn Iterator<Item = &'a T> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.items
    }
}

impl SemaItem for ResolvedStruct {
    fn id(&self) -> ItemId {
        self.id
    }
    fn module_path(&self) -> String {
        self.module_path.to_string()
    }
    fn attrs(&self) -> &[syn::Attribute] {
        &self.node.attrs
    }
    fn file(&self) -> &std::path::Path {
        &self.file
    }
    fn name(&self) -> String {
        self.node.ident.to_string()
    }
    fn is_public(&self) -> bool {
        matches!(self.node.vis, syn::Visibility::Public(_))
    }
}

impl SemaItem for ResolvedFunction {
    fn id(&self) -> ItemId {
        self.id
    }
    fn module_path(&self) -> String {
        self.module_path.to_string()
    }
    fn attrs(&self) -> &[syn::Attribute] {
        &self.node.attrs
    }
    fn file(&self) -> &std::path::Path {
        &self.file
    }
    fn name(&self) -> String {
        self.node.sig.ident.to_string()
    }
    fn is_public(&self) -> bool {
        matches!(self.node.vis, syn::Visibility::Public(_))
    }
}

impl SemaItem for ResolvedEnum {
    fn id(&self) -> ItemId {
        self.id
    }
    fn module_path(&self) -> String {
        self.module_path.to_string()
    }
    fn attrs(&self) -> &[syn::Attribute] {
        &self.node.attrs
    }
    fn file(&self) -> &std::path::Path {
        &self.file
    }
    fn name(&self) -> String {
        self.node.ident.to_string()
    }
    fn is_public(&self) -> bool {
        matches!(self.node.vis, syn::Visibility::Public(_))
    }
}

impl SemaItem for ResolvedTrait {
    fn id(&self) -> ItemId {
        self.id
    }
    fn module_path(&self) -> String {
        self.module_path.to_string()
    }
    fn attrs(&self) -> &[syn::Attribute] {
        &self.node.attrs
    }
    fn file(&self) -> &std::path::Path {
        &self.file
    }
    fn name(&self) -> String {
        self.node.ident.to_string()
    }
    fn is_public(&self) -> bool {
        matches!(self.node.vis, syn::Visibility::Public(_))
    }
}

impl ResolvedTrait {
    pub fn impl_blocks<'a>(
        &self,
        workspace: &'a crate::workspaces::Workspace,
    ) -> Vec<&'a ResolvedImpl> {
        workspace.impls_of_trait(self.id)
    }
}

impl ResolvedStruct {
    pub fn implementations<'a>(
        &self,
        workspace: &'a crate::workspaces::Workspace,
    ) -> Vec<&'a ResolvedImpl> {
        self.impls(workspace)
    }
}

impl SemaItem for ResolvedImpl {
    fn id(&self) -> ItemId {
        self.id
    }
    fn module_path(&self) -> String {
        self.module_path.to_string()
    }
    fn attrs(&self) -> &[syn::Attribute] {
        &self.node.attrs
    }
    fn file(&self) -> &std::path::Path {
        &self.file
    }
    fn name(&self) -> String {
        format!("impl{}", self.module_path)
    }
    fn is_public(&self) -> bool {
        //FIXME: impls don't have visibility
        false
    }
}
