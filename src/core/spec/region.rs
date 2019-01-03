use {
    core::{
        parse::Tree,
        spec,
    },
    std::{
        collections::HashSet,
        error,
        fmt,
    },
};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum RegionType {
    Alphabet,
    CDFA,
    Grammar,
}

lazy_static! {
    static ref REQUIRED_REGIONS: Vec<RegionType> = vec![
        RegionType::Alphabet,
        RegionType::CDFA,
        RegionType::Grammar,
    ];
}

pub fn traverse(
    regions_node: &Tree,
    handler: &mut FnMut(&Tree, &RegionType) -> Result<(), spec::GenError>,
) -> Result<(), spec::GenError> {
    let mut region_types: HashSet<RegionType> = HashSet::new();

    traverse_regions_node(regions_node, handler, &mut region_types)?;

    for region_type in REQUIRED_REGIONS.iter() {
        if !region_types.contains(&region_type) {
            return Err(spec::GenError::from(Error::MissingRequired(region_type.clone())));
        }
    }

    Ok(())
}

fn traverse_regions_node(
    regions_node: &Tree,
    handler: &mut FnMut(&Tree, &RegionType) -> Result<(), spec::GenError>,
    region_types: &mut HashSet<RegionType>,
) -> Result<(), spec::GenError> {
    if regions_node.children.len() == 2 {
        traverse_regions_node(
            regions_node.get_child(0),
            handler,
            region_types,
        )?;
    }

    traverse_region_node(
        regions_node.children.last().unwrap(),
        handler,
        region_types,
    )
}

fn traverse_region_node(
    region_node: &Tree,
    handler: &mut FnMut(&Tree, &RegionType) -> Result<(), spec::GenError>,
    region_types: &mut HashSet<RegionType>,
) -> Result<(), spec::GenError> {
    let inner_node = region_node.get_child(0);
    let region_type = type_from_node(region_node);

    handler(inner_node, &region_type)?;
    region_types.insert(region_type);

    Ok(())
}

fn type_from_node(region_node: &Tree) -> RegionType {
    let region_type_string = &region_node.get_child(0).lhs.kind;
    match &region_type_string[..] {
        "alphabet" => RegionType::Alphabet,
        "cdfa" => RegionType::CDFA,
        "grammar" => RegionType::Grammar,
        &_ => panic!("Invalid specification region type: '{}'", region_type_string)
    }
}


#[derive(Debug)]
pub enum Error {
    MissingRequired(RegionType),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::MissingRequired(ref region) =>
                write!(f, "Missing required region: '{:?}'", region),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::MissingRequired(_) => None,
        }
    }
}
