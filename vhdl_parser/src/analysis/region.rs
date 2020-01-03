// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2018, Olof Kraigher olof.kraigher@gmail.com
use crate::ast::*;
use crate::data::*;

use fnv::FnvHashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

#[derive(Clone)]
pub enum AnyDeclaration {
    AliasOf(Box<AnyDeclaration>),
    Other,
    Overloaded,
    // An optional region with implicit declarations
    TypeDeclaration(Option<Arc<Region<'static>>>),
    IncompleteType,
    Constant,
    DeferredConstant,
    // The region of the protected type which needs to be extendend by the body
    ProtectedType(Arc<Region<'static>>),
    ProtectedTypeBody,
    Library(Symbol),
    Entity(UnitId, Arc<Region<'static>>),
    Configuration(UnitId, Arc<Region<'static>>),
    Package(UnitId, Arc<Region<'static>>),
    UninstPackage(UnitId, Arc<Region<'static>>),
    PackageInstance(UnitId, Arc<Region<'static>>),
    Context(UnitId, Arc<Region<'static>>),
    LocalPackageInstance(Symbol, Arc<Region<'static>>),
}

impl AnyDeclaration {
    pub fn from_object_declaration(decl: &ObjectDeclaration) -> AnyDeclaration {
        match decl {
            ObjectDeclaration {
                class: ObjectClass::Constant,
                ref expression,
                ..
            } => {
                if expression.is_none() {
                    AnyDeclaration::DeferredConstant
                } else {
                    AnyDeclaration::Constant
                }
            }
            _ => AnyDeclaration::Other,
        }
    }

    fn is_deferred_constant(&self) -> bool {
        if let AnyDeclaration::DeferredConstant = self {
            true
        } else {
            false
        }
    }

    fn is_non_deferred_constant(&self) -> bool {
        if let AnyDeclaration::Constant = self {
            true
        } else {
            false
        }
    }

    fn is_protected_type(&self) -> bool {
        if let AnyDeclaration::ProtectedType(..) = self {
            true
        } else {
            false
        }
    }

    fn is_protected_type_body(&self) -> bool {
        if let AnyDeclaration::ProtectedTypeBody = self {
            true
        } else {
            false
        }
    }

    fn is_incomplete_type(&self) -> bool {
        if let AnyDeclaration::IncompleteType = self {
            true
        } else {
            false
        }
    }

    fn is_type_declaration(&self) -> bool {
        if let AnyDeclaration::TypeDeclaration(..) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
struct AnyDeclarationData {
    /// The location where the declaration was made
    /// Builtin and implicit declaration will not have a source position
    decl_pos: Option<SrcPos>,
    decl: AnyDeclaration,
}

impl AnyDeclarationData {
    fn error(&self, diagnostics: &mut dyn DiagnosticHandler, message: impl Into<String>) {
        if let Some(ref pos) = self.decl_pos {
            diagnostics.push(Diagnostic::error(pos, message));
        }
    }

    fn hint(&self, diagnostics: &mut dyn DiagnosticHandler, message: impl Into<String>) {
        if let Some(ref pos) = self.decl_pos {
            diagnostics.push(Diagnostic::hint(pos, message));
        }
    }
}

#[derive(Clone)]
pub struct VisibleDeclaration {
    pub designator: Designator,
    data: Vec<AnyDeclarationData>,
}

impl VisibleDeclaration {
    pub fn new(
        designator: impl Into<WithPos<Designator>>,
        decl: AnyDeclaration,
    ) -> VisibleDeclaration {
        let designator = designator.into();

        VisibleDeclaration {
            designator: designator.item,
            data: vec![AnyDeclarationData {
                decl_pos: Some(designator.pos),
                decl,
            }],
        }
    }

    fn first_data(&self) -> &AnyDeclarationData {
        self.data
            .first()
            .expect("Declaration always contains one entry")
    }

    pub fn first(&self) -> &AnyDeclaration {
        &self.first_data().decl
    }

    pub fn first_pos(&self) -> Option<&SrcPos> {
        self.first_data().decl_pos.as_ref()
    }

    pub fn second(&self) -> Option<&AnyDeclaration> {
        self.data.get(1).map(|data| &data.decl)
    }

    pub fn is_overloaded(&self) -> bool {
        if let AnyDeclaration::Overloaded = self.first_data().decl {
            true
        } else {
            false
        }
    }

    /// Return a duplicate declaration of the previous declaration if it exists
    fn find_duplicate_of<'a>(&self, prev_decl: &'a Self) -> Option<&'a AnyDeclarationData> {
        if self.is_overloaded() && prev_decl.is_overloaded() {
            return None;
        }

        let ref later_decl = self.first();
        for prev_decl_data in prev_decl.data.iter() {
            let ref prev_decl = prev_decl_data.decl;

            match prev_decl {
                // Everything expect deferred combinations are forbidden
                AnyDeclaration::DeferredConstant if later_decl.is_non_deferred_constant() => {}
                AnyDeclaration::ProtectedType(..) if later_decl.is_protected_type_body() => {}
                AnyDeclaration::IncompleteType if later_decl.is_type_declaration() => {}
                _ => {
                    return Some(prev_decl_data);
                }
            }
        }
        None
    }
}
#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(test, derive(Debug))]
enum RegionKind {
    PackageDeclaration,
    PackageBody,
    Other,
}

impl Default for RegionKind {
    fn default() -> RegionKind {
        RegionKind::Other
    }
}

#[derive(Clone, Default)]
pub struct Region<'a> {
    parent: Option<&'a Region<'a>>,
    extends: Option<&'a Region<'a>>,
    visible: FnvHashMap<Designator, VisibleDeclaration>,
    decls: FnvHashMap<Designator, VisibleDeclaration>,
    kind: RegionKind,
}

impl<'a> Region<'a> {
    pub fn default() -> Region<'static> {
        Region {
            parent: None,
            extends: None,
            visible: FnvHashMap::default(),
            decls: FnvHashMap::default(),
            kind: RegionKind::Other,
        }
    }

    pub fn nested(&'a self) -> Region<'a> {
        Region {
            parent: Some(self),
            extends: None,
            visible: FnvHashMap::default(),
            decls: FnvHashMap::default(),
            kind: RegionKind::Other,
        }
    }

    pub fn without_parent(self) -> Region<'static> {
        Region {
            parent: None,
            extends: None,
            visible: self.visible,
            decls: self.decls,
            kind: self.kind,
        }
    }

    pub fn in_package_declaration(mut self) -> Region<'a> {
        self.kind = RegionKind::PackageDeclaration;
        self
    }

    pub fn extend(region: &'a Region<'a>, parent: Option<&'a Region<'a>>) -> Region<'a> {
        let kind = match region.kind {
            RegionKind::PackageDeclaration => RegionKind::PackageBody,
            _ => RegionKind::Other,
        };

        Region {
            parent: parent,
            extends: Some(region),
            visible: FnvHashMap::default(),
            decls: FnvHashMap::default(),
            kind,
        }
    }

    pub fn close_immediate(&mut self, diagnostics: &mut dyn DiagnosticHandler) {
        assert!(self.extends.is_none());
        self.check_incomplete_types_are_defined(diagnostics);
    }

    /// Incomplete types must be defined in the same immediate region as they are declared
    fn check_incomplete_types_are_defined(&self, diagnostics: &mut dyn DiagnosticHandler) {
        for decl in self.decls.values() {
            if decl.first().is_incomplete_type() {
                let mut check_ok = false;
                if let Some(second) = decl.second() {
                    if second.is_type_declaration() {
                        check_ok = true;
                    }
                }

                if !check_ok {
                    decl.first_data().error(
                        diagnostics,
                        format!(
                            "Missing full type declaration of incomplete type '{}'",
                            &decl.designator
                        ),
                    );
                    decl.first_data().hint(diagnostics, "The full type declaration shall occur immediately within the same declarative part");
                }
            }
        }
    }

    fn check_deferred_constant_pairs(&self, diagnostics: &mut dyn DiagnosticHandler) {
        match self.kind {
            // Package without body may not have deferred constants
            RegionKind::PackageDeclaration => {
                for decl in self.decls.values() {
                    match decl.first() {
                        AnyDeclaration::DeferredConstant => {
                            decl.first_data().error(diagnostics, format!("Deferred constant '{}' lacks corresponding full constant declaration in package body", &decl.designator));
                        }
                        _ => {}
                    }
                }
            }
            RegionKind::PackageBody => {
                let ref extends = self
                    .extends
                    .as_ref()
                    .expect("Package body must extend package");
                for ext_decl in extends.decls.values() {
                    match ext_decl.first() {
                        AnyDeclaration::DeferredConstant => {
                            // Deferred constants may only be located in a package
                            // And only matched with a constant in the body
                            let mut found = false;
                            let decl = self.decls.get(&ext_decl.designator);

                            if let Some(decl) = decl {
                                if let AnyDeclaration::Constant = decl.first() {
                                    found = true;
                                }
                            }

                            if !found {
                                ext_decl.first_data().error(diagnostics, format!("Deferred constant '{}' lacks corresponding full constant declaration in package body", &ext_decl.designator));
                            }
                        }
                        _ => {}
                    }
                }
            }
            RegionKind::Other => {}
        }
    }

    fn check_protected_types_have_body(&self, diagnostics: &mut dyn DiagnosticHandler) {
        for decl in self.decls.values() {
            if decl.first().is_protected_type() {
                if let Some(second_decl) = decl.second() {
                    if second_decl.is_protected_type_body() {
                        continue;
                    }
                }
                decl.first_data().error(
                    diagnostics,
                    format!("Missing body for protected type '{}'", &decl.designator),
                );
            }
        }

        if let Some(ref extends) = self.extends {
            for ext_decl in extends.decls.values() {
                if ext_decl.first().is_protected_type() {
                    if let Some(second_decl) = ext_decl.second() {
                        if second_decl.is_protected_type_body() {
                            continue;
                        }
                    }

                    if let Some(decl) = self.decls.get(&ext_decl.designator) {
                        if decl.first().is_protected_type_body() {
                            continue;
                        }
                    }
                    ext_decl.first_data().error(
                        diagnostics,
                        format!("Missing body for protected type '{}'", &ext_decl.designator),
                    );
                }
            }
        }
    }

    #[must_use]
    fn check_deferred_constant_only_in_package(
        &self,
        decl: &VisibleDeclaration,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> bool {
        if self.kind != RegionKind::PackageDeclaration && decl.first().is_deferred_constant() {
            decl.first_data().error(
                diagnostics,
                "Deferred constants are only allowed in package declarations (not body)",
            );
            false
        } else {
            true
        }
    }

    #[must_use]
    fn check_full_constand_of_deferred_only_in_body(
        &self,
        decl: &VisibleDeclaration,
        prev_decl: &VisibleDeclaration,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> bool {
        if self.kind != RegionKind::PackageBody && decl.first().is_non_deferred_constant() {
            if prev_decl.first().is_deferred_constant() {
                decl.first_data().error(
                    diagnostics,
                    "Full declaration of deferred constant is only allowed in a package body",
                );
                return false;
            }
        }
        true
    }

    pub fn close_both(&mut self, diagnostics: &mut dyn DiagnosticHandler) {
        self.check_incomplete_types_are_defined(diagnostics);
        self.check_deferred_constant_pairs(diagnostics);
        self.check_protected_types_have_body(diagnostics);
    }

    /// Check duplicate declarations
    /// Allow deferred constants, incomplete types and protected type bodies
    /// Returns true if the declaration does not duplicates an existing declaration
    #[must_use]
    fn check_duplicate(
        decl: &VisibleDeclaration,
        prev_decl: &VisibleDeclaration,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> bool {
        if let Some(duplicate_decl) = decl.find_duplicate_of(&prev_decl) {
            if let Some(ref pos) = decl.first_data().decl_pos {
                let mut diagnostic = Diagnostic::error(
                    pos,
                    format!("Duplicate declaration of '{}'", decl.designator),
                );

                if let Some(ref prev_pos) = duplicate_decl.decl_pos {
                    diagnostic.add_related(prev_pos, "Previously defined here");
                }

                diagnostics.push(diagnostic)
            }
            false
        } else {
            true
        }
    }

    /// true if the declaration can be added
    fn check_add(
        // The declaration to add
        decl: &VisibleDeclaration,
        // Previous declaration in the same region
        prev_decl: Option<&VisibleDeclaration>,
        // Previous declaration in the region extended by this region
        ext_decl: Option<&VisibleDeclaration>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> bool {
        let mut check_ok = true;

        if let Some(prev_decl) = prev_decl {
            if !Self::check_duplicate(&decl, &prev_decl, diagnostics) {
                check_ok = false;
            }
        }

        if let Some(ext_decl) = ext_decl {
            if !Self::check_duplicate(&decl, &ext_decl, diagnostics) {
                check_ok = false;
            }
        }

        check_ok
    }

    fn add_decl(&mut self, decl: VisibleDeclaration, diagnostics: &mut dyn DiagnosticHandler) {
        let ext_decl = self
            .extends
            .as_ref()
            .and_then(|extends| extends.decls.get(&decl.designator));

        if !self.check_deferred_constant_only_in_package(&decl, diagnostics) {
            return;
        }

        if let Some(ext_decl) = ext_decl {
            if !self.check_full_constand_of_deferred_only_in_body(&decl, ext_decl, diagnostics) {
                return;
            }
        }

        // @TODO merge with .entry below
        if let Some(prev_decl) = self.decls.get(&decl.designator) {
            if !self.check_full_constand_of_deferred_only_in_body(&decl, prev_decl, diagnostics) {
                return;
            }
        }

        match self.decls.entry(decl.designator.clone()) {
            Entry::Occupied(ref mut entry) => {
                let prev_decl = entry.get_mut();

                if Self::check_add(&decl, Some(&prev_decl), ext_decl, diagnostics) {
                    let mut decl = decl;
                    prev_decl.data.append(&mut decl.data);
                }
            }
            Entry::Vacant(entry) => {
                if Self::check_add(&decl, None, ext_decl, diagnostics) {
                    entry.insert(decl);
                }
            }
        }
    }

    pub fn add(
        &mut self,
        designator: impl Into<WithPos<Designator>>,
        decl: AnyDeclaration,
        diagnostics: &mut dyn DiagnosticHandler,
    ) {
        let decl = VisibleDeclaration::new(designator, decl);
        self.add_decl(decl, diagnostics);
    }

    pub fn overwrite(&mut self, designator: impl Into<WithPos<Designator>>, decl: AnyDeclaration) {
        let decl = VisibleDeclaration::new(designator.into(), decl);
        self.decls.insert(decl.designator.clone(), decl);
    }

    pub fn add_implicit(
        &mut self,
        designator: impl Into<Designator>,
        decl_pos: Option<&SrcPos>,
        decl: AnyDeclaration,
        diagnostics: &mut dyn DiagnosticHandler,
    ) {
        let decl = VisibleDeclaration {
            designator: designator.into(),
            data: vec![AnyDeclarationData {
                decl_pos: decl_pos.cloned(),
                decl,
            }],
        };
        self.add_decl(decl, diagnostics);
    }

    pub fn make_library_visible(
        &mut self,
        designator: impl Into<Designator>,
        library_name: &Symbol,
        decl_pos: Option<SrcPos>,
    ) {
        let decl = VisibleDeclaration {
            designator: designator.into(),
            data: vec![AnyDeclarationData {
                decl_pos: decl_pos.clone(),
                decl: AnyDeclaration::Library(library_name.clone()),
            }],
        };
        self.make_potentially_visible(decl);
    }

    /// Add implicit declarations when using declaration
    /// For example all enum literals are made implicititly visible when using an enum type
    fn add_implicit_declarations(&mut self, decl: &AnyDeclaration) {
        match decl {
            AnyDeclaration::TypeDeclaration(ref implicit) => {
                // Add implicitic declarations when using type
                if let Some(implicit) = implicit {
                    self.make_all_potentially_visible(&implicit);
                }
            }
            AnyDeclaration::AliasOf(ref decl) => {
                self.add_implicit_declarations(decl);
            }
            _ => {}
        }
    }

    pub fn make_potentially_visible(&mut self, decl: impl Into<VisibleDeclaration>) {
        let decl = decl.into();
        self.add_implicit_declarations(decl.first());
        self.visible.insert(decl.designator.clone(), decl);
    }

    pub fn make_all_potentially_visible(&mut self, region: &Region<'a>) {
        for decl in region.decls.values() {
            self.make_potentially_visible(decl.clone());
        }
    }

    /// Used when using context clauses
    pub fn copy_visibility_from(&mut self, region: &Region<'a>) {
        for decl in region.visible.values() {
            self.make_potentially_visible(decl.clone());
        }
    }

    /// Helper function lookup a visible declaration within the region
    fn lookup(&self, designator: &Designator, is_selected: bool) -> Option<&VisibleDeclaration> {
        self.decls
            .get(designator)
            .or_else(|| {
                self.extends
                    .as_ref()
                    .and_then(|region| region.lookup(designator, is_selected))
            })
            .or_else(|| {
                if is_selected {
                    None
                } else {
                    self.visible.get(designator)
                }
            })
            .or_else(|| {
                if is_selected {
                    None
                } else {
                    self.parent
                        .as_ref()
                        .and_then(|region| region.lookup_within(designator))
                }
            })
    }

    /// Lookup where this region is the prefix of a selected name
    /// Thus any visibility inside the region is irrelevant
    pub fn lookup_selected(&self, designator: &Designator) -> Option<&VisibleDeclaration> {
        self.lookup(designator, true)
    }

    /// Lookup a designator from within the region itself
    /// Thus all parent regions and visibility is relevant
    pub fn lookup_within(&self, designator: &Designator) -> Option<&VisibleDeclaration> {
        self.lookup(designator, false)
    }
}

pub trait SetReference {
    fn set_reference(&mut self, decl: &VisibleDeclaration) {
        // @TODO handle built-ins without position
        // @TODO handle mutliple overloaded declarations
        if !decl.is_overloaded() {
            // We do not set references to overloaded names to avoid
            // To much incorrect behavior which will appear as low quality
            self.set_reference_pos(decl.first_pos());
        } else {
            self.clear_reference();
        }
    }

    fn clear_reference(&mut self) {
        self.set_reference_pos(None);
    }

    fn set_reference_pos(&mut self, pos: Option<&SrcPos>);
}

impl<T> SetReference for WithRef<T> {
    fn set_reference_pos(&mut self, pos: Option<&SrcPos>) {
        self.reference = pos.cloned();
    }
}

impl<T: SetReference> SetReference for WithPos<T> {
    fn set_reference_pos(&mut self, pos: Option<&SrcPos>) {
        self.item.set_reference_pos(pos);
    }
}
