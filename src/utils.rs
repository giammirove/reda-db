use eyre::{eyre, OptionExt, Result};
use num_traits::{Float, FromPrimitive, Signed, ToPrimitive, Zero};
use reda_lefdef::{DEFNet, DEFPin, LEFDEFOrient, LEFMacro, DEF, LEF};
use std::{
    collections::HashMap,
    ops::{AddAssign, Rem, SubAssign},
};

pub trait Numeric:
    Float
    + Zero
    + Sized
    + Send
    + Sync
    + Rem
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
    pub fn new(pin_id: usize, macro_id: usize, offset: Coords<T>, size: Size<T>) -> Self {
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
    pub fn new(pin_ids: Vec<usize>) -> Self {
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
    pub fn push(&mut self, x: T, y: T) {
        self.x.push(x);
        self.y.push(y);
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
    pub fn push(&mut self, w: T, h: T) {
        self.w.push(w);
        self.h.push(h);
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
    pub fn new(
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
    pub fn new(x: T, y: T) -> Self {
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
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum InstanceType {
    MACRO,
    #[default]
    STDCELL,
    TERMINAL,
}
impl InstanceType {
    pub(crate) fn is_macro(&self) -> bool {
        match self {
            InstanceType::MACRO => true,
            _ => false,
        }
    }
    pub(crate) fn is_terminal(&self) -> bool {
        match self {
            InstanceType::TERMINAL => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum PlacementType {
    FIXED,
    #[default]
    MOVABLE,
}
impl PlacementType {
    pub(crate) fn is_movable(&self) -> bool {
        match self {
            PlacementType::MOVABLE => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum OrientType {
    #[default]
    N,
    S,
    E,
    W,
    FN,
    FS,
    FE,
    FW,
}
impl OrientType {
    pub(crate) fn from(orient: &LEFDEFOrient) -> Self {
        match orient {
            LEFDEFOrient::N => OrientType::N,
            LEFDEFOrient::S => OrientType::S,
            LEFDEFOrient::E => OrientType::E,
            LEFDEFOrient::W => OrientType::W,
            LEFDEFOrient::FN => OrientType::FN,
            LEFDEFOrient::FS => OrientType::FS,
            LEFDEFOrient::FE => OrientType::FE,
            LEFDEFOrient::FW => OrientType::FW,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ParsedInstance<'a, T: Numeric> {
    /// ID of the Instance
    pub(crate) id: usize,
    pub(crate) name: &'a str,
    pub(crate) model: &'a str,

    /// Instance Type
    _instance_type: InstanceType,
    /// Placement Type
    _placement_type: PlacementType,
    /// Orient Type
    _orient_type: OrientType,

    /// Initial coordinates
    pub(crate) _coords: Coords<T>,
    /// Offset from initial coordinates
    pub(crate) _offset: Coords<T>,

    /// Size of the instance (as define in the LEF)
    pub(crate) _size: Size<T>,
}
impl<'a, T: Numeric> ParsedInstance<'a, T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: usize,
        name: &'a str,
        model: &'a str,
        _instance_type: InstanceType,
        _placement_type: PlacementType,
        _orient_type: OrientType,
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
            _orient_type,
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
    #[allow(unused)]
    pub(crate) site_name: Option<String>,
    pub size: Size<T>,
    full_size: Size<T>,
    // offset of the site
    pub offset: Coords<T>,
}
impl<T: Numeric> DieArea<T> {
    pub fn new(
        site_name: Option<String>,
        size: Size<T>,
        full_size: Size<T>,
        offset: Coords<T>,
    ) -> Self {
        Self {
            site_name,
            size,
            full_size,
            offset,
        }
    }
    pub fn width(&self) -> T {
        self.size.width
    }
    pub fn height(&self) -> T {
        self.size.height
    }
    pub fn full_width(&self) -> T {
        self.full_size.width
    }
    pub fn full_height(&self) -> T {
        self.full_size.height
    }
    pub fn area(&self) -> T {
        self.size.width * self.size.height
    }
    pub fn use_full_size(&mut self) {
        self.size = Size::new(self.full_size.width, self.full_size.height);
    }
}

#[derive(Debug, Default)]
pub struct VecInstances<T: Numeric> {
    pub coords: VecCoords<T>,
    pub sizes: VecSizes<T>,
    pub(crate) types: Vec<InstanceType>,
    pub(crate) pl_types: Vec<PlacementType>,
    pub(crate) o_types: Vec<OrientType>,
    pub areas: Vec<T>,    // redundant info but useful for computation
    pub num_pins: Vec<T>, // use T instead of usize to avoid later conversions
}
impl<T: Numeric> VecInstances<T> {
    pub fn new(
        coords: VecCoords<T>,
        sizes: VecSizes<T>,
        types: Vec<InstanceType>,
        pl_types: Vec<PlacementType>,
        o_types: Vec<OrientType>,
        areas: Vec<T>,
        num_pins: Vec<T>,
    ) -> Self {
        assert!(coords.len() == sizes.len());
        assert!(types.len() == sizes.len());
        assert!(pl_types.len() == sizes.len());
        assert!(o_types.len() == sizes.len());
        assert!(areas.len() == sizes.len());
        assert!(num_pins.len() == sizes.len());
        Self {
            coords,
            sizes,
            types,
            pl_types,
            o_types,
            areas,
            num_pins,
        }
    }

    pub fn push(
        &mut self,
        x: T,
        y: T,
        w: T,
        h: T,
        t: InstanceType,
        pl_t: PlacementType,
        o_t: OrientType,
        num_pins: T,
    ) -> usize {
        self.coords.push(x, y);
        self.sizes.push(w, h);
        self.types.push(t);
        self.pl_types.push(pl_t);
        self.o_types.push(o_t);
        self.areas.push(w * h);
        self.num_pins.push(num_pins);

        self.coords.len() - 1
    }

    pub fn len(&self) -> usize {
        self.coords.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_movable(&self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }
        self.pl_types[index].is_movable()
    }
    pub fn is_macro(&self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }
        self.types[index].is_macro()
    }
    pub fn is_terminal(&self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }
        self.types[index].is_terminal()
    }
}

type RawPin<T> = (usize, Coords<T>, Size<T>);

pub(crate) fn get_lef_units<'a>(lef: &'a LEF) -> Result<i32> {
    let lef_units_raw = lef
        .units
        .as_ref()
        .and_then(|u| u.database)
        .ok_or_eyre("Missing LEF database units")?;

    Ok(lef_units_raw)
}

pub(crate) fn get_lef_def_units<'a, T: Numeric>(lef: &'a LEF, def: &'a DEF) -> Result<T> {
    let lef_units_raw = lef
        .units
        .as_ref()
        .and_then(|u| u.database)
        .ok_or_eyre("Missing LEF database units")?;

    let def_units_raw = def.units.ok_or_eyre("Missing DEF units")?;

    let lef_units = T::from(lef_units_raw).ok_or_eyre("Failed to convert LEF units")?;

    let def_units = T::from(def_units_raw).ok_or_eyre("Failed to convert DEF units")?;

    Ok(lef_units / def_units)
}

pub(crate) fn get_scale<'a, T: Numeric>(lef: &'a LEF) -> Result<T> {
    let lef_units = get_lef_units(lef)?;
    let site = lef.get_site();
    match site {
        None => T::from(0.005),
        Some(site) => {
            if let Some(size) = &site.opts.size {
                T::from(1. / (size.width * (lef_units as f32)))
            } else {
                T::from(0.005)
            }
        }
    }
    .ok_or_eyre("Error while computing scale")
}

pub(crate) fn get_lef_scale<'a, T: Numeric>(lef: &'a LEF) -> Result<T> {
    let scale = get_scale(lef)?;
    Ok(T::from(get_lef_units(lef)?).unwrap() * scale)
}

pub(crate) fn parse_io_pin<'a, T: Numeric>(
    name_to_defpin: &HashMap<&str, &DEFPin>,
    pin_name: &str,
    units: T,
    scale: T,
    instances: &mut VecInstances<T>,
) -> Result<RawPin<T>> {
    let one = T::one();

    let lib = name_to_defpin
        .get(pin_name)
        .ok_or_eyre(format!("IO PIN NOT FOUND IN DEF {}", pin_name))?;

    let (px, py, orient_type) = if let Some(ref pl) = lib.opts.placement {
        (
            T::from(pl.location.x).unwrap() * units * scale,
            T::from(pl.location.y).unwrap() * units * scale,
            OrientType::from(&pl.orient),
        )
    } else {
        return Err(eyre!("PLACEMENT NOT FOUND IN PIN {}", pin_name));
    };

    let bbox = lib.get_bbox()?;
    let bbox_w = T::from(bbox.w).unwrap();
    let bbox_h = T::from(bbox.h).unwrap();

    // Use integer midpoint to match DREAMPLACE truncating division
    let half_w = T::from(bbox.w as i32 / 2).unwrap();
    let half_h = T::from(bbox.h as i32 / 2).unwrap();

    let inst_x = px + T::from(bbox.xl).unwrap() * units * scale;
    let inst_y = py + T::from(bbox.yl).unwrap() * units * scale;

    let (pin_x, pin_y) = match orient_type {
        OrientType::E | OrientType::W | OrientType::FE | OrientType::FW => {
            (half_h * units * scale, half_w * units * scale) // swap x/y
        }
        _ => (half_w * units * scale, half_h * units * scale),
    };
    let pin_coords = Coords::new(pin_x, pin_y);

    let fake_instance_id = instances.push(
        inst_x,
        inst_y,
        T::from(bbox.w).unwrap() * units * scale,
        T::from(bbox.h).unwrap() * units * scale,
        InstanceType::TERMINAL,
        PlacementType::FIXED,
        orient_type,
        one,
    );

    let pin_size = Size::new(bbox_w * units * scale, bbox_h * units * scale);
    Ok((fake_instance_id, pin_coords, pin_size))
}

pub(crate) fn parse_node_pin<'a, T: Numeric>(
    name_to_lib: &HashMap<&str, &LEFMacro>,
    inst_name_to_cell: &HashMap<&str, &ParsedInstance<T>>,
    pin_name: &str,
    inst_name: &str,
    units: T,
    lef_scale: T,
) -> Result<RawPin<T>> {
    let two = T::one() + T::one();
    let cell = inst_name_to_cell
        .get(inst_name)
        .ok_or_eyre(format!("INSTANCE NOT FOUND IN DEF {}", inst_name))?;
    let model = cell.model;
    let lib = name_to_lib
        .get(model)
        .ok_or_eyre(format!("INSTANCE MODEL NOT FOUND IN LEF {}", model))?;
    let lib_pin = lib
        .get_pin(pin_name)
        .ok_or_eyre(format!("PIN {} NOT FOUND IN CELL {}", pin_name, model))?;

    let bbox = lib_pin.get_bbox()?;
    let bbox_x = T::from(bbox.xl).unwrap();
    let bbox_y = T::from(bbox.yl).unwrap();
    let bbox_w = T::from(bbox.w).unwrap();
    let bbox_h = T::from(bbox.h).unwrap();

    // ======================================================================
    // DREAMPlace applies rotation then removes it => just dont do it then
    // ======================================================================

    // Raw pin center in cell-local space (no orientation applied)
    // DREAMPlace stores raw offsets and rotates in convertOrient post-pass
    let local_x = (bbox_x + bbox_w / two) * units * lef_scale;
    let local_y = (bbox_y + bbox_h / two) * units * lef_scale;

    let size = Size::new(bbox_w * units * lef_scale, bbox_h * units * lef_scale);
    let coords = Coords::new(local_x, local_y);

    Ok((cell.id, coords, size))
}

pub(crate) fn parse_net<'a, T: Numeric>(
    name_to_defpin: &HashMap<&str, &DEFPin>,
    name_to_lib: &HashMap<&str, &LEFMacro>,
    inst_name_to_cell: &HashMap<&str, &ParsedInstance<T>>,
    net: &DEFNet,
    units: T,
    scale: T,
    lef_scale: T,

    instances: &mut VecInstances<T>,
    temp_2nets: &mut Vec<Vec<RawPin<T>>>,
    temp_3nets: &mut Vec<Vec<RawPin<T>>>,
    temp_nnets: &mut Vec<Vec<RawPin<T>>>,
) -> Result<()> {
    if let Some(ref conns) = net.opts.connections {
        let mut pins = vec![];
        for conn in conns {
            let inst_name: &str = &conn.0;
            let pin_name: &str = &conn.1;
            // TODO: IO PIN
            let (cell_id, coords, size) = if inst_name == "PIN" {
                parse_io_pin(&name_to_defpin, pin_name, units, scale, instances)?
            } else {
                parse_node_pin(
                    &name_to_lib,
                    &inst_name_to_cell,
                    pin_name,
                    inst_name,
                    units,
                    lef_scale,
                )?
            };

            let pin = (cell_id, coords, size);
            pins.push(pin);
        }

        match pins.len() {
            // not valid
            0 | 1 => {}
            2 => temp_2nets.push(pins),
            3 => temp_3nets.push(pins),
            _ => temp_nnets.push(pins),
        }
    } else {
        log::info!("NET IS EMPTY {}", net.name);
    }

    Ok(())
}
