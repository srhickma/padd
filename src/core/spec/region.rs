use {
    core::{
        parse::Tree,
        spec::{self, lang::SpecSymbol},
    },
    std::{collections::HashSet, error, fmt},
};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum RegionType {
    Injectable,
    Ignorable,
    Alphabet,
    CDFA,
    Grammar,
}

lazy_static! {
    /// The list of required regions in a specification.
    static ref REQUIRED_REGIONS: Vec<RegionType> = vec![RegionType::CDFA, RegionType::Grammar];
}

/// Recursively traverses the specification regions under `regions_node`, calling `handler` with
/// the `SpecSymbol::Region` node and type of each region.
///
/// Returns an error if a required specification region is missing, or if `handler` returns an
/// error for any traversed region.
pub fn traverse(
    regions_node: &Tree<SpecSymbol>,
    handler: &mut dyn FnMut(&Tree<SpecSymbol>, &RegionType) -> Result<(), spec::GenError>,
) -> Result<(), spec::GenError> {
    let mut region_types: HashSet<RegionType> = HashSet::new();

    traverse_regions_node(regions_node, handler, &mut region_types)?;

    for region_type in REQUIRED_REGIONS.iter() {
        if !region_types.contains(&region_type) {
            return Err(spec::GenError::from(Error::MissingRequired(
                region_type.clone(),
            )));
        }
    }

    Ok(())
}

/// Recursively traverses the specification regions under `regions_node`, calling `handler` with
/// the `SpecSymbol::Region` node and type of each region, and storing the types of visited regions
/// in the `region_types` accumulator.
///
/// Returns an error if `handler` returns an error for any traversed region.
fn traverse_regions_node(
    regions_node: &Tree<SpecSymbol>,
    handler: &mut dyn FnMut(&Tree<SpecSymbol>, &RegionType) -> Result<(), spec::GenError>,
    region_types: &mut HashSet<RegionType>,
) -> Result<(), spec::GenError> {
    if regions_node.children.len() == 2 {
        traverse_regions_node(regions_node.get_child(0), handler, region_types)?;
    }

    traverse_region_node(regions_node.children.last().unwrap(), handler, region_types)
}

/// Traverses a single specification region represented by `region_node`, calling `handler` with
/// the associated `SpecSymbol::Region` node and type of the region, and storing the type in the
/// `region_types` accumulator.
///
/// Returns an error if `handler` returns an error for the traversed region.
fn traverse_region_node(
    region_node: &Tree<SpecSymbol>,
    handler: &mut dyn FnMut(&Tree<SpecSymbol>, &RegionType) -> Result<(), spec::GenError>,
    region_types: &mut HashSet<RegionType>,
) -> Result<(), spec::GenError> {
    let inner_node = region_node.get_child(0);
    let region_type = type_from_node(region_node);

    handler(inner_node, &region_type)?;
    region_types.insert(region_type);

    Ok(())
}

/// Returns the region type associated with the `SpecSymbol::Regions` node `regions_node`.
fn type_from_node(region_node: &Tree<SpecSymbol>) -> RegionType {
    let region_symbol = &region_node.get_child(0).lhs.kind();
    match region_symbol {
        SpecSymbol::Injectable => RegionType::Injectable,
        SpecSymbol::Ignorable => RegionType::Ignorable,
        SpecSymbol::Alphabet => RegionType::Alphabet,
        SpecSymbol::CDFA => RegionType::CDFA,
        SpecSymbol::Grammar => RegionType::Grammar,
        &_ => panic!("Invalid specification region type: '{:?}'", region_symbol),
    }
}

/// Error: Represents an error encountered when traversing specification regions.
///
/// # Types
///
/// * `MissingRequired` - indicates that a required region is not present.
#[derive(Debug)]
pub enum Error {
    MissingRequired(RegionType),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::MissingRequired(ref region) => {
                write!(f, "Missing required region: '{:?}'", region)
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::MissingRequired(_) => None,
        }
    }
}
