use eyre::{eyre, OptionExt, Result};
use num_traits::{Float, FromPrimitive, Signed, ToPrimitive, Zero};
use reda_lefdef::{read_def, read_lef, DEF, LEF};
use std::fmt;
use std::{collections::HashMap, ffi::OsString, ops::AddAssign, ops::SubAssign};

const SCALE: f32 = 1.;

pub trait Numeric:
    Float
    + Zero
    + Sized
    + Send
    + Sync
    + std::iter::Sum
    + AddAssign
    + SubAssign
    + FromPrimitive
    + ToPrimitive
    + Signed
    + std::fmt::Debug
    + std::fmt::LowerExp
    + std::default::Default
    + 'static
{
}
impl<T> Numeric for T where
    T: Float
        + Zero
        + Sized
        + Send
        + Sync
        + std::iter::Sum
        + AddAssign
        + SubAssign
        + FromPrimitive
        + ToPrimitive
        + Signed
        + std::fmt::Debug
        + std::fmt::LowerExp
        + std::default::Default
        + 'static
{
}

#[derive(Debug)]
pub struct ParsedPin<T: Numeric> {
    /// ID of Pin
    pub pin_id: usize,
    /// ID of the Macro of this Pin
    pub macro_id: usize,
    /// Offset from Macro
    pub offset: Coords<T>,
    /// Size of the Pin
    pub size: Size<T>,
}
impl<T: Numeric> ParsedPin<T> {
    fn new(pin_id: usize, macro_id: usize, offset: Coords<T>, size: Size<T>) -> Self {
        Self {
            pin_id,
            macro_id,
            offset,
            size,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Net {
    pub pin_ids: Vec<usize>,
}
impl Net {
    fn new(pin_ids: Vec<usize>) -> Self {
        Self { pin_ids }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct VecCoordsIter<'a, T> {
    x: std::slice::Iter<'a, T>,
    y: std::slice::Iter<'a, T>,
}
impl<'a, T> Iterator for VecCoordsIter<'a, T> {
    type Item = (&'a T, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match (self.x.next(), self.y.next()) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.x.size_hint()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct VecCoordsIterMut<'a, T> {
    x: std::slice::IterMut<'a, T>,
    y: std::slice::IterMut<'a, T>,
}
impl<'a, T> Iterator for VecCoordsIterMut<'a, T> {
    type Item = (&'a mut T, &'a mut T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match (self.x.next(), self.y.next()) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.x.size_hint()
    }
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct VecCoords<T: Numeric> {
    pub x: Vec<T>,
    pub y: Vec<T>,
}
impl<T: Numeric> VecCoords<T> {
    pub fn new(x: Vec<T>, y: Vec<T>) -> Self {
        Self { x, y }
    }
    pub fn new_zero(num: usize) -> Self {
        let zero = T::zero();
        Self {
            x: vec![zero; num],
            y: vec![zero; num],
        }
    }
    pub fn len(&self) -> usize {
        assert!(self.x.len() == self.y.len());
        self.x.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn iter(&self) -> VecCoordsIter<'_, T> {
        assert_eq!(self.x.len(), self.y.len());
        VecCoordsIter {
            x: self.x.iter(),
            y: self.y.iter(),
        }
    }
    pub fn iter_mut(&mut self) -> VecCoordsIterMut<'_, T> {
        assert_eq!(self.x.len(), self.y.len());
        VecCoordsIterMut {
            x: self.x.iter_mut(),
            y: self.y.iter_mut(),
        }
    }
}
impl<'a, T: Numeric> IntoIterator for &'a VecCoords<T> {
    type Item = (&'a T, &'a T);
    type IntoIter = VecCoordsIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T: Numeric> IntoIterator for &'a mut VecCoords<T> {
    type Item = (&'a mut T, &'a mut T);
    type IntoIter = VecCoordsIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct VecSizesIter<'a, T> {
    w: std::slice::Iter<'a, T>,
    h: std::slice::Iter<'a, T>,
}
impl<'a, T> Iterator for VecSizesIter<'a, T> {
    type Item = (&'a T, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match (self.w.next(), self.h.next()) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.w.size_hint()
    }
}
#[derive(Debug)]
#[repr(C)]
pub struct VecSizesIterMut<'a, T> {
    w: std::slice::IterMut<'a, T>,
    h: std::slice::IterMut<'a, T>,
}
impl<'a, T> Iterator for VecSizesIterMut<'a, T> {
    type Item = (&'a mut T, &'a mut T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match (self.w.next(), self.h.next()) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.w.size_hint()
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct VecSizes<T> {
    pub w: Vec<T>,
    pub h: Vec<T>,
}
impl<T> VecSizes<T> {
    pub fn new(w: Vec<T>, h: Vec<T>) -> Self {
        Self { w, h }
    }
    pub fn len(&self) -> usize {
        assert!(self.w.len() == self.h.len());
        self.w.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn iter(&self) -> VecSizesIter<'_, T> {
        assert_eq!(self.w.len(), self.h.len());
        VecSizesIter {
            w: self.w.iter(),
            h: self.h.iter(),
        }
    }
    pub fn iter_mut(&mut self) -> VecSizesIterMut<'_, T> {
        assert_eq!(self.w.len(), self.h.len());
        VecSizesIterMut {
            w: self.w.iter_mut(),
            h: self.h.iter_mut(),
        }
    }
}
impl<'a, T> IntoIterator for &'a VecSizes<T> {
    type Item = (&'a T, &'a T);
    type IntoIter = VecSizesIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut VecSizes<T> {
    type Item = (&'a mut T, &'a mut T);
    type IntoIter = VecSizesIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Debug)]
pub struct Netlist<T: Numeric> {
    pub parsed_pins: Vec<ParsedPin<T>>,
    pub pins: VecCoords<T>,

    pub pin_2_macro: Vec<usize>,
    pub macro_2_pins: Vec<Vec<usize>>,

    pub nets: Vec<Net>,
    /// Number of Nets with 2 pins
    pub n_2nets: usize,
    /// Number of Nets with 3 pins
    pub n_3nets: usize,
    /// Number of Nets with > 3 pins
    pub n_nnets: usize,
}
impl<T: Numeric> Netlist<T> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        parsed_pins: Vec<ParsedPin<T>>,
        pins: VecCoords<T>,
        pin_2_macro: Vec<usize>,
        macro_2_pins: Vec<Vec<usize>>,
        nets: Vec<Net>,
        n_2nets: usize,
        n_3nets: usize,
        n_nnets: usize,
    ) -> Self {
        Self {
            parsed_pins,
            pins,
            pin_2_macro,
            macro_2_pins,
            nets,
            n_2nets,
            n_3nets,
            n_nnets,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct Coords<T> {
    pub x: T,
    pub y: T,
}
impl<T> Coords<T> {
    fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}
impl<T> Size<T> {
    fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum InstanceType {
    MACRO,
    STDCELL,
}

#[derive(Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum PlacementType {
    FIXED,
    MOVABLE,
}

#[derive(Debug)]
pub(crate) struct ParsedInstance<'a, T: Numeric> {
    /// ID of the Instance
    id: usize,
    name: &'a str,
    model: &'a str,

    /// Instance Type
    _instance_type: InstanceType,
    /// Placement Type
    _placement_type: PlacementType,

    /// Initial coordinates
    _coords: Coords<T>,
    /// Offset from initial coordinates
    _offset: Coords<T>,

    /// Size of the instance (as define in the LEF)
    _size: Size<T>,
}
impl<'a, T: Numeric> ParsedInstance<'a, T> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: usize,
        name: &'a str,
        model: &'a str,
        _instance_type: InstanceType,
        _placement_type: PlacementType,
        _coords: Coords<T>,
        _offset: Coords<T>,
        _size: Size<T>,
    ) -> Self {
        Self {
            id,
            name,
            model,
            _instance_type,
            _placement_type,
            _coords,
            _offset,
            _size,
        }
    }

    pub(crate) fn update_id(&mut self, id: usize) {
        self.id = id;
    }
}

#[derive(Debug)]
pub struct DieArea<T: Numeric> {
    site_name: Option<String>,
    size: Size<T>,
    // offset of the site
    pub offset: Coords<T>,
}
impl<T: Numeric> DieArea<T> {
    fn new(site_name: Option<String>, size: Size<T>, offset: Coords<T>) -> Self {
        Self {
            site_name,
            size,
            offset,
        }
    }
    pub fn width(&self) -> T {
        self.size.width
    }
    pub fn height(&self) -> T {
        self.size.height
    }
    pub fn area(&self) -> T {
        self.size.width * self.size.height
    }
}

#[derive(Debug, Default)]
pub struct VecInstances<T: Numeric> {
    pub coords: VecCoords<T>,
    pub sizes: VecSizes<T>,
    pub areas: Vec<T>,    // redundant info but useful for computation
    pub num_pins: Vec<T>, // use T instead of usize to avoid later conversions
}
impl<T: Numeric> VecInstances<T> {
    fn new(coords: VecCoords<T>, sizes: VecSizes<T>, areas: Vec<T>, num_pins: Vec<T>) -> Self {
        Self {
            coords,
            sizes,
            areas,
            num_pins,
        }
    }
    pub fn len(&self) -> usize {
        self.coords.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct DB<T: Numeric> {
    pub diearea: DieArea<T>,
    pub instances: VecInstances<T>,
    pub netlist: Netlist<T>,

    pub num_movable: usize,
    pub movable_area: T,
    pub fixed_area: T,
    pub cell_utilization: T,
}
impl<T: Numeric> DB<T> {
    fn new(
        diearea: DieArea<T>,
        instances: VecInstances<T>,
        netlist: Netlist<T>,
        num_movable: usize,
    ) -> Self {
        let mut movable_area = T::zero();
        let mut fixed_area = T::zero();

        for i in 0..num_movable {
            movable_area += instances.areas[i];
        }
        for i in num_movable..instances.len() {
            fixed_area += instances.areas[i];
        }

        let cell_utilization = movable_area / (diearea.area() - fixed_area);

        Self {
            diearea,
            instances,
            netlist,
            num_movable,
            movable_area,
            fixed_area,
            cell_utilization,
        }
    }
}

impl<T: Numeric> fmt::Display for DB<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n_nets = self.netlist.nets.len();

        writeln!(f, "NETLIST")?;
        writeln!(f, "  PINS: {}", self.netlist.pins.len())?;
        writeln!(f, "  NETS: {}", n_nets)?;
        writeln!(
            f,
            "    2-NETS: {} - {}%",
            self.netlist.n_2nets,
            self.netlist.n_2nets * 100 / n_nets
        )?;
        writeln!(
            f,
            "    3-NETS: {} - {}%",
            self.netlist.n_3nets,
            self.netlist.n_3nets * 100 / n_nets
        )?;
        writeln!(
            f,
            "    n-NETS: {} - {}%",
            self.netlist.n_nnets,
            self.netlist.n_nnets * 100 / n_nets
        )?;
        writeln!(
            f,
            "  DIEAREA: {:?} - {:?}",
            self.diearea.size, self.diearea.offset
        )?;
        writeln!(f, "  MOVABLE AREA: {:?}", self.movable_area)?;
        writeln!(f, "  FIXED AREA: {:?}", self.fixed_area)?;
        writeln!(f, "  NUM MOVABLE: {:?}", self.num_movable)?;
        writeln!(f, "  CELL UTILIZATION: {:?}", self.cell_utilization)?;

        Ok(())
    }
}

fn read_instances<'a, T: Numeric>(
    lef: &'a LEF,
    def: &'a DEF,
    diearea: &DieArea<T>,
) -> Result<(Vec<ParsedInstance<'a, T>>, VecInstances<T>, usize)> {
    let units = T::from(def.units.unwrap()).unwrap();
    let scale = T::from(SCALE).unwrap();

    let name_to_lib: HashMap<&str, _> = lef
        .macros
        .iter()
        .flatten()
        .map(|m| (m.name.as_str(), m))
        .collect();

    let zero = T::zero();
    let one = T::one();

    let mut parsed_instances = Vec::new();
    let mut fixed_parsed_instances = Vec::new();

    let mut instances_x = Vec::new();
    let mut instances_y = Vec::new();
    let mut instances_w = Vec::new();
    let mut instances_h = Vec::new();
    let mut instances_area = Vec::new();
    let mut instances_num_pins = Vec::new();

    let mut fixed_instances_x = Vec::new();
    let mut fixed_instances_y = Vec::new();
    let mut fixed_instances_w = Vec::new();
    let mut fixed_instances_h = Vec::new();
    let mut fixed_instances_area = Vec::new();
    let mut fixed_instances_num_pins = Vec::new();

    let offset = Coords::new(zero, zero);
    let mut id = 0;

    let comps = match def.components.as_ref() {
        Some(c) => c,
        None => return Ok((parsed_instances, VecInstances::default(), 0)),
    };
    for comp in comps {
        let name = comp.name.as_str();
        let model = comp.model.as_str();

        let comp_lib = name_to_lib
            .get(model)
            .unwrap_or_else(|| panic!("INSTANCE MODEL NOT FOUND IN LEF {model}"));

        let coords = match comp.opts.placement.as_ref() {
            Some(p) => {
                let (offset_x, offset_y) = if comp_lib.is_in_site(diearea.site_name.clone()) {
                    (diearea.offset.x, diearea.offset.y)
                } else {
                    (T::zero(), T::zero())
                };

                let coords = Coords::new(
                    ((T::from(p.1.x).unwrap() - offset_x) / units) * scale,
                    ((T::from(p.1.y).unwrap() - offset_y) / units) * scale,
                );

                assert!(
                    coords.x >= zero,
                    "negative x {:?} (originally {:?}) with offset {:?} for {}",
                    coords.x,
                    p.1.x,
                    offset_x,
                    name
                );
                assert!(
                    coords.y >= zero,
                    "negative y {:?} (originally {:?}) with offset {:?} for {}",
                    coords.y,
                    p.1.y,
                    offset_y,
                    name
                );

                coords
            }
            None => Coords::new(T::zero(), T::zero()),
        };

        let placement_type = if comp.is_fixed() {
            PlacementType::FIXED
        } else {
            PlacementType::MOVABLE
        };

        let instance_type = if comp_lib.is_macro() {
            InstanceType::MACRO
        } else {
            InstanceType::STDCELL
        };

        let size = comp_lib.opts.size.as_ref().expect("SIZE NOT FOUND");
        let new_size = Size::new(
            T::from(size.width).unwrap() * scale,
            T::from(size.height).unwrap() * scale,
        );

        let num_pins = T::from(comp_lib.opts.pins.as_ref().map_or(0, |pins| pins.len())).unwrap();

        let push_instance = |xs: &mut Vec<T>,
                             ys: &mut Vec<T>,
                             ws: &mut Vec<T>,
                             hs: &mut Vec<T>,
                             areas: &mut Vec<T>,
                             pins: &mut Vec<T>| {
            xs.push(coords.x);
            ys.push(coords.y);
            ws.push(new_size.width);
            hs.push(new_size.height);
            areas.push(new_size.width * new_size.height);
            pins.push(num_pins);
        };

        match placement_type {
            PlacementType::FIXED => {
                push_instance(
                    &mut fixed_instances_x,
                    &mut fixed_instances_y,
                    &mut fixed_instances_w,
                    &mut fixed_instances_h,
                    &mut fixed_instances_area,
                    &mut fixed_instances_num_pins,
                );

                fixed_parsed_instances.push(ParsedInstance::new(
                    0,
                    name,
                    model,
                    instance_type,
                    placement_type,
                    coords,
                    offset,
                    new_size,
                ));
            }
            PlacementType::MOVABLE => {
                push_instance(
                    &mut instances_x,
                    &mut instances_y,
                    &mut instances_w,
                    &mut instances_h,
                    &mut instances_area,
                    &mut instances_num_pins,
                );

                parsed_instances.push(ParsedInstance::new(
                    id,
                    name,
                    model,
                    instance_type,
                    placement_type,
                    coords,
                    offset,
                    new_size,
                ));

                id += 1;
            }
        }
    }

    // IO pin instance
    fixed_instances_x.push(zero);
    fixed_instances_y.push(zero);
    fixed_instances_w.push(zero);
    fixed_instances_h.push(zero);
    fixed_instances_area.push(zero);
    fixed_instances_num_pins.push(one);

    instances_x.extend(fixed_instances_x);
    instances_y.extend(fixed_instances_y);
    instances_w.extend(fixed_instances_w);
    instances_h.extend(fixed_instances_h);
    instances_area.extend(fixed_instances_area);
    instances_num_pins.extend(fixed_instances_num_pins);

    let num_movable = id;

    for fixed in &mut fixed_parsed_instances {
        fixed.update_id(id);
        id += 1;
    }

    fixed_parsed_instances.push(ParsedInstance::new(
        id,
        "IO",
        "IO",
        InstanceType::MACRO,
        PlacementType::FIXED,
        Coords::new(zero, zero),
        Coords::new(zero, zero),
        Size::new(zero, zero),
    ));

    parsed_instances.extend(fixed_parsed_instances);

    let instances = VecInstances::new(
        VecCoords::new(instances_x, instances_y),
        VecSizes::new(instances_w, instances_h),
        instances_area,
        instances_num_pins,
    );

    assert_eq!(parsed_instances.len(), instances.len());

    Ok((parsed_instances, instances, num_movable))
}

fn read_diearea<T: Numeric>(lef: &LEF, def: &DEF) -> Result<DieArea<T>> {
    let scale = T::from(SCALE).unwrap();

    // site
    let site = lef
        .sites
        .as_ref()
        .and_then(|sites| sites.iter().find(|s| s.is_core()).or_else(|| sites.first()));

    // offsets
    let (site_offset_x, site_offset_y) = match site {
        None => (0.0, 0.0),
        Some(site) => def.rows.as_ref().map_or((0.0, 0.0), |rows| {
            let filtered: Vec<_> = rows.iter().filter(|r| r.site == site.name).collect();

            if filtered.is_empty() {
                (0.0, 0.0)
            } else {
                let min_x = filtered
                    .iter()
                    .map(|r| r.x as f32)
                    .fold(f32::INFINITY, f32::min);

                let min_y = filtered
                    .iter()
                    .map(|r| r.y as f32)
                    .fold(f32::INFINITY, f32::min);

                (min_x, min_y)
            }
        }),
    };

    // diearea
    let diearea = def
        .diearea
        .as_ref()
        .ok_or_eyre("DIEAREA not found in DEF")?;

    let units = def.units.ok_or_eyre("UNITS not found in DEF")?;

    let units_t = T::from(units).unwrap();

    let (offset_x, offset_y, diearea_width, diearea_height) = diearea.get_rectangle()?;

    let die_offset_x = site_offset_x + offset_x;
    let die_offset_y = site_offset_y + offset_y;
    let site_name = site.map(|s| s.name.clone());

    Ok(DieArea::new(
        site_name,
        Size::new(
            (T::from(diearea_width - site_offset_x * 2.).unwrap() / units_t) * scale,
            (T::from(diearea_height - site_offset_y * 2.).unwrap() / units_t) * scale,
        ),
        Coords::new(
            T::from(die_offset_x).unwrap(),
            T::from(die_offset_y).unwrap(),
        ),
    ))
}

fn read_netlist<'a, T: Numeric>(
    lef: &LEF,
    def: &DEF,
    parsed_instances: &Vec<ParsedInstance<'a, T>>,
    instances: &VecInstances<T>,
) -> Result<Netlist<T>> {
    let units = T::from(def.units.unwrap()).unwrap();
    let scale = T::from(SCALE).unwrap();
    let zero = T::zero();
    // last one
    let iopin_id = parsed_instances.len() - 1;

    let name_to_lib: HashMap<&str, _> = lef
        .macros
        .iter()
        .flatten()
        .map(|macr| (macr.name.as_str(), macr))
        .collect();

    let inst_name_to_cell: HashMap<&str, _> = parsed_instances
        .iter()
        .map(|macr| (macr.name, macr))
        .collect();

    let name_to_iopin: HashMap<&str, _> = def
        .pins
        .iter()
        .flatten()
        .map(|pin| (pin.name.as_str(), pin))
        .collect();

    // temporary vectors with pins
    // temporary because the pin_id must match the final order (2-net,3-net,n-net)
    let mut temp_2nets = vec![];
    let mut temp_3nets = vec![];
    let mut temp_nnets = vec![];

    if let Some(ref def_nets) = def.nets {
        for net in def_nets {
            let mut pins = vec![];

            if let Some(ref conns) = net.opts.connections {
                for conn in conns {
                    let inst_name: &str = &conn.0;
                    let pin_name: &str = &conn.1;
                    // TODO: IO PIN
                    let (cell_id, coords, size) = if inst_name == "PIN" {
                        let lib = name_to_iopin
                            .get(pin_name)
                            .unwrap_or_else(|| panic!("IO PIN NOT FOUND IN DEF {}", pin_name));

                        let (px, py) = if let Some(ref pl) = lib.opts.placement {
                            (pl.location.x, pl.location.y)
                        } else {
                            // panic!("PLACEMENT NOT FOUND IN PIN {}", pin_name);
                            // CLK in some cases might be be placed
                            (0, 0)
                        };

                        let coords = Coords::new(
                            (T::from(px).unwrap() / units) * scale,
                            (T::from(py).unwrap() / units) * scale,
                        );
                        let size = Size::new(zero, zero);

                        (iopin_id, coords, size)
                    } else {
                        let cell = inst_name_to_cell
                            .get(inst_name)
                            .unwrap_or_else(|| panic!("INSTANCE NOT FOUND IN DEF {}", inst_name));

                        let model = cell.model;

                        let lib = name_to_lib
                            .get(model)
                            .unwrap_or_else(|| panic!("INSTANCE MODEL NOT FOUND IN LEF {}", model));

                        let lib_pin = lib.get_pin(pin_name).unwrap_or_else(|| {
                            panic!("PIN {} NOT FOUND IN CELL {}", pin_name, model)
                        });

                        if let Some(ref lib_pin_ports) = lib_pin.opts.ports {
                            // TODO: expand to more complex shapes
                            let port = lib_pin_ports.first().ok_or(eyre!("not enough ports"))?;
                            let rect = port.get_rect()?;
                            let coords = Coords::new(
                                (T::from(rect.0).unwrap() / units) * scale,
                                (T::from(rect.1).unwrap() / units) * scale,
                            );
                            let size = Size::new(
                                T::from(rect.2).unwrap() * scale,
                                T::from(rect.3).unwrap() * scale,
                            );

                            (cell.id, coords, size)
                        } else {
                            panic!("PORTS NOT FOUND IN PIN {} OF CELL {}", pin_name, model);
                        }
                    };

                    let pin = (cell_id, coords, size);
                    pins.push(pin);
                }

                match pins.len() {
                    0 | 1 => {}
                    2 => temp_2nets.push(pins),
                    3 => temp_3nets.push(pins),
                    _ => temp_nnets.push(pins),
                }
            } else {
                log::info!("NET IS EMPTY {}", net.name);
            }
        }
    }

    let mut pin_id: usize = 0;
    let mut parsed_pins = vec![];
    let mut pins_x = vec![];
    let mut pins_y = vec![];
    let mut pin_2_macro = vec![];
    let mut macro_2_pins = vec![vec![]; parsed_instances.len()];
    let mut nets = vec![];

    let n_2nets = temp_2nets.len();
    let n_3nets = temp_3nets.len();
    let n_nnets = temp_nnets.len();

    for net_pins in temp_2nets
        .iter()
        .chain(temp_3nets.iter())
        .chain(temp_nnets.iter())
    {
        let net_pin_ids: Vec<usize> = net_pins
            .iter()
            .map(|pin| {
                let id = pin_id;
                let macro_id = pin.0;
                let coords = pin.1;
                pins_x.push(coords.x);
                pins_y.push(coords.y);
                parsed_pins.push(ParsedPin::new(id, macro_id, coords, pin.2.clone()));
                pin_2_macro.push(macro_id);
                macro_2_pins[macro_id].push(id);
                pin_id += 1;
                id
            })
            .collect();

        nets.push(Net::new(net_pin_ids));
    }

    // sanity check
    let num_instance_pins: T = instances.num_pins.iter().copied().sum();
    assert!(
        T::from(parsed_pins.len()).unwrap() <= num_instance_pins,
        " Pins from nets {} <!= Pins from instances {:?} ",
        parsed_pins.len(),
        num_instance_pins,
    );

    let pins = VecCoords::new(pins_x, pins_y);

    Ok(Netlist::new(
        parsed_pins,
        pins,
        pin_2_macro,
        macro_2_pins,
        nets,
        n_2nets,
        n_3nets,
        n_nnets,
    ))
}

pub fn read_db<T: Numeric>(
    lef_path: &OsString,
    def_path: &OsString,
    verbose: bool,
) -> Result<DB<T>> {
    let lef = read_lef(lef_path)?;
    let def = read_def(def_path)?;

    let diearea = read_diearea(&lef, &def)?;
    let (parsed_instances, instances, num_movable) = read_instances(&lef, &def, &diearea)?;
    let netlist = read_netlist(&lef, &def, &parsed_instances, &instances)?;

    let db = DB::new(diearea, instances, netlist, num_movable);

    if verbose {
        log::info!("{}", lef);
        log::info!("{}", def);
        log::info!("{}", db);
    }

    Ok(db)
}
