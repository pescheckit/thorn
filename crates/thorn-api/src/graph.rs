use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The complete application graph — models, fields, relations, and settings
/// extracted from a running framework process.
///
/// This is framework-agnostic: the same structure represents Django models,
/// Laravel Eloquent models, Rails ActiveRecord models, etc. The bridge
/// crate for each framework populates this from runtime introspection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppGraph {
    pub models: Vec<Model>,
    pub installed_apps: Vec<String>,
    pub settings: FrameworkSettings,
}

impl AppGraph {
    /// Look up a model by app/namespace + name (case-insensitive on name).
    pub fn get_model(&self, app: &str, name: &str) -> Option<&Model> {
        self.models
            .iter()
            .find(|m| m.app_label == app && m.name.eq_ignore_ascii_case(name))
    }

    /// Find a model by name only (first match).
    pub fn find_model_by_name(&self, name: &str) -> Option<&Model> {
        self.models
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// Find all models with a given name (handles ambiguous names across apps).
    pub fn find_models_by_name(&self, name: &str) -> Vec<&Model> {
        self.models
            .iter()
            .filter(|m| m.name.eq_ignore_ascii_case(name))
            .collect()
    }

    /// All models in a given app/namespace.
    pub fn models_in_app<'a>(&'a self, app: &'a str) -> impl Iterator<Item = &'a Model> {
        self.models.iter().filter(move |m| m.app_label == app)
    }

    /// Find all models that have a FK/O2O/BelongsTo pointing at a given model.
    pub fn models_pointing_to(&self, app: &str, name: &str) -> Vec<&Model> {
        self.models
            .iter()
            .filter(|m| {
                m.relations.iter().any(|r| {
                    r.to_model_app == app
                        && r.to_model == name
                        && matches!(r.kind, RelationKind::ForeignKey | RelationKind::OneToOne)
                })
            })
            .collect()
    }
}

/// Framework-level settings extracted from the runtime.
///
/// Uses a generic key-value `extra` map for framework-specific settings.
/// Common patterns (auth user model, databases, middleware) have first-class fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrameworkSettings {
    /// The configured user model (e.g. "auth.User" in Django, "App\\Models\\User" in Laravel).
    pub auth_user_model: String,
    /// Configured database connections.
    pub databases: Vec<String>,
    /// Middleware/pipeline stack.
    pub middleware: Vec<String>,
    /// Framework-specific settings as key-value pairs.
    pub extra: HashMap<String, String>,
}

/// A model/entity in the application — represents a Django Model, Eloquent Model,
/// ActiveRecord model, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// App or namespace (e.g. "auth", "App\\Models", "users").
    pub app_label: String,
    /// Model class name (e.g. "User", "Post").
    pub name: String,
    /// Database table name.
    pub db_table: String,
    /// Source module name (e.g. "django.contrib.auth.models").
    pub module: String,
    /// Source file path (e.g. "pescheck_core/models/user.py"). Empty if unknown.
    #[serde(default)]
    pub source_file: String,
    /// Whether this is an abstract base model.
    pub abstract_model: bool,
    /// Whether this is a proxy/STI model.
    pub proxy: bool,
    /// Concrete fields on this model.
    pub fields: Vec<Field>,
    /// Relations (FK, O2O, M2M, BelongsTo, HasMany, etc.).
    pub relations: Vec<Relation>,
    /// Managers/scopes/query builders.
    pub managers: Vec<Manager>,
    /// Parent class names (for model inheritance).
    pub parents: Vec<String>,
    /// Public method names on the model class.
    pub methods: Vec<String>,
}

impl Model {
    /// Check if this model defines a method with the given name.
    pub fn has_method(&self, name: &str) -> bool {
        self.methods.iter().any(|m| m == name)
    }

    /// Get a field by name.
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get a relation by name.
    pub fn get_relation(&self, name: &str) -> Option<&Relation> {
        self.relations.iter().find(|r| r.name == name)
    }

    /// All field names (concrete fields + relations).
    pub fn all_field_names(&self) -> Vec<&str> {
        self.fields
            .iter()
            .map(|f| f.name.as_str())
            .chain(self.relations.iter().map(|r| r.name.as_str()))
            .collect()
    }

    /// Check if a field or relation name exists on this model.
    pub fn has_field_or_relation(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.name == name) || self.relations.iter().any(|r| r.name == name)
    }

    /// Get the default manager (or first manager if none marked default).
    pub fn default_manager(&self) -> Option<&Manager> {
        self.managers
            .iter()
            .find(|m| m.is_default)
            .or_else(|| self.managers.first())
    }
}

/// A concrete field on a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    /// Database column name.
    pub column: String,
    /// Full field class path (e.g. "django.db.models.CharField", "string").
    pub field_class: String,
    /// The language-native type this field maps to (e.g. "str", "int", "string", "integer").
    pub native_type: String,
    pub nullable: bool,
    pub blank: bool,
    pub default: Option<String>,
    pub max_length: Option<i64>,
    pub choices: Vec<(String, String)>,
    pub validators: Vec<String>,
    pub primary_key: bool,
    pub unique: bool,
    pub db_index: bool,
}

/// A relation between models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub name: String,
    pub kind: RelationKind,
    /// Target model name (e.g. "User", "Post").
    pub to_model: String,
    /// Target model's app/namespace.
    pub to_model_app: String,
    /// Reverse accessor name on the target model (e.g. "book_set", "posts").
    pub related_name: String,
    /// Query name for filtering (e.g. "book", "post").
    pub related_query_name: String,
    /// Deletion behavior (e.g. "CASCADE", "SET_NULL", "restrict").
    pub on_delete: Option<String>,
    pub nullable: bool,
    /// Intermediate/pivot/through model for M2M.
    pub through_model: Option<String>,
}

/// The kind of relation between models — uses terms common across ORMs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationKind {
    /// Django ForeignKey, Laravel BelongsTo, Rails belongs_to
    ForeignKey,
    /// Django OneToOneField, Laravel HasOne, Rails has_one
    OneToOne,
    /// Django ManyToManyField, Laravel BelongsToMany, Rails has_and_belongs_to_many
    ManyToMany,
    /// Reverse FK accessor (e.g. book_set, posts)
    Reverse,
    /// Reverse O2O accessor
    ReverseOneToOne,
}

/// A manager, scope, or query builder on a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manager {
    /// Name of the manager accessor (e.g. "objects", "active", "published").
    pub name: String,
    /// Full class path of the manager.
    pub manager_class: String,
    /// Full class path of the queryset class.
    pub queryset_class: String,
    pub is_default: bool,
    /// Custom methods defined on this manager/scope.
    pub custom_methods: Vec<String>,
}

// ── Backward-compatible type aliases ─────────────────────────────────────
// These keep existing code (thorn-bridge, thorn-django) compiling without changes.

/// Alias for backward compatibility.
pub type ModelGraph = AppGraph;
/// Alias for backward compatibility.
pub type DjangoModel = Model;
/// Alias for backward compatibility.
pub type DjangoSettings = FrameworkSettings;
/// Alias for backward compatibility.
pub type FieldDef = Field;
/// Alias for backward compatibility.
pub type RelationDef = Relation;
/// Alias for backward compatibility.
pub type ManagerDef = Manager;
