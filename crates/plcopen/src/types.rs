use serde::{Deserialize, Serialize};

// ─── Projet ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub file_header: FileHeader,
    pub content_header: ContentHeader,
    pub pous: Vec<Pou>,
    pub data_types: Vec<UserDataType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileHeader {
    pub company_name: Option<String>,
    pub product_name: Option<String>,
    pub product_version: Option<String>,
    pub creation_date_time: Option<String>,
    pub last_modified_date_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentHeader {
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub organization: Option<String>,
    pub description: Option<String>,
}

// ─── POU ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pou {
    pub name: String,
    pub pou_type: PouType,
    pub interface: Interface,
    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PouType {
    Program,
    FunctionBlock,
    Function,
}

// ─── Interface / Variables ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Interface {
    pub return_type: Option<DataTypeRef>,
    pub input_vars: Vec<Variable>,
    pub output_vars: Vec<Variable>,
    pub in_out_vars: Vec<Variable>,
    pub local_vars: Vec<Variable>,
    pub temp_vars: Vec<Variable>,
    pub external_vars: Vec<Variable>,
    pub global_vars: Vec<Variable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub data_type: DataTypeRef,
    pub initial_value: Option<String>,
    pub comment: Option<String>,
    pub retain: bool,
    pub constant: bool,
}

/// Référence à un type de donnée (IEC 61131-3 + types dérivés).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "of")]
pub enum DataTypeRef {
    // Booléen
    Bool,
    // Entiers signés
    SInt,
    Int,
    DInt,
    LInt,
    // Entiers non signés
    USInt,
    UInt,
    UDInt,
    ULInt,
    // Flottants
    Real,
    LReal,
    // Chaînes
    String,
    WString,
    // Bit-strings
    Byte,
    Word,
    DWord,
    LWord,
    // Temps
    Time,
    Date,
    TimeOfDay,
    DateAndTime,
    // Type dérivé (DDT, FB, etc.)
    Derived(std::string::String),
    // Tableau
    Array {
        base_type: Box<DataTypeRef>,
        dimensions: Vec<ArrayDimension>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArrayDimension {
    pub lower: i64,
    pub upper: i64,
}

/// Type de donnée utilisateur (DDT/UDT) — stub, enrichi si besoin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataType {
    pub name: String,
}

// ─── Corps du POU ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "lang")]
pub enum Body {
    Ld(LdBody),
    Fbd(FbdBody),
    St(StBody),
    Il(IlBody),
    Sfc(SfcBody),
}

// ─── Ladder Diagram (LD) ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdBody {
    pub networks: Vec<LdNetwork>,
}

/// Un réseau = un échelon ladder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdNetwork {
    pub local_id: u32,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub elements: Vec<LdElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LdElement {
    LeftPowerRail(LeftPowerRail),
    RightPowerRail(RightPowerRail),
    Contact(Contact),
    Coil(Coil),
    Block(LdBlock),
}

// ─── Éléments graphiques communs ──────────────────────────────────────────────

/// Coordonnées absolues de l'élément dans le réseau.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Coordonnées relatives d'un point de connexion par rapport à son élément.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelPosition {
    pub x: i32,
    pub y: i32,
}

/// Référence vers un autre élément du réseau via son `localId`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub ref_local_id: u32,
    pub formal_parameter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionPointIn {
    pub rel_position: RelPosition,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionPointOut {
    pub rel_position: RelPosition,
    pub formal_parameter: Option<String>,
}

// ─── Rails d'alimentation ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeftPowerRail {
    pub local_id: u32,
    pub position: Position,
    pub height: u32,
    pub connection_points_out: Vec<ConnectionPointOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightPowerRail {
    pub local_id: u32,
    pub position: Position,
    pub height: u32,
    pub connection_points_in: Vec<ConnectionPointIn>,
}

// ─── Contact ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub local_id: u32,
    pub position: Position,
    /// `true` = contact normalement fermé (NF / NC).
    pub negated: bool,
    pub edge: EdgeDetection,
    pub variable: String,
    pub connection_point_in: Option<ConnectionPointIn>,
    pub connection_point_out: Option<ConnectionPointOut>,
}

/// Détection de front sur un contact ou une bobine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum EdgeDetection {
    #[default]
    None,
    Rising,
    Falling,
    Both,
}

// ─── Bobine (Coil) ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coil {
    pub local_id: u32,
    pub position: Position,
    /// `true` = bobine normalement fermée (sortie inversée).
    pub negated: bool,
    pub storage: CoilStorage,
    pub edge: CoilEdge,
    pub variable: String,
    pub connection_point_in: Option<ConnectionPointIn>,
    pub connection_point_out: Option<ConnectionPointOut>,
}

/// Mode de mémorisation d'une bobine (SET, RESET, rétentif…).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CoilStorage {
    #[default]
    None,
    Set,
    Reset,
    Retentive,
    NonRetentive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CoilEdge {
    #[default]
    None,
    Rising,
    Falling,
}

// ─── Bloc fonctionnel en LD ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdBlock {
    pub local_id: u32,
    pub position: Position,
    /// Nom du type de bloc (ex : "TON", "CTU", "ADD", "MOVE"…).
    pub type_name: String,
    /// Nom de l'instance (ex : "Timer1").
    pub instance_name: Option<String>,
    pub execution_order_id: Option<u32>,
    pub inputs: Vec<BlockPin>,
    pub outputs: Vec<BlockPin>,
}

/// Broche (pin) d'un bloc fonctionnel avec son point de connexion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPin {
    pub formal_parameter: String,
    pub rel_position: RelPosition,
    pub connections: Vec<Connection>,
}

// ─── Structured Text (ST) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StBody {
    pub text: String,
}

// ─── Instruction List (IL) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IlBody {
    pub text: String,
}

// ─── Function Block Diagram (FBD) — stub ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbdBody {
    pub networks: Vec<FbdNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbdNetwork {
    pub local_id: u32,
    pub comment: Option<String>,
    pub blocks: Vec<LdBlock>,
}

// ─── Sequential Function Chart (SFC) — stub ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfcBody {
    /// Contenu XML brut du corps SFC (à enrichir dans une prochaine itération).
    pub raw_xml: String,
}
