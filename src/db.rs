use eyre::{OptionExt, Result};
use reda_lefdef::{read_def, read_lef, DEFPin, DEF, LEF};
use std::fmt;
use std::{collections::HashMap, ffi::OsString};

use crate::utils::{
    get_lef_def_units, get_lef_scale, get_scale, parse_net, Coords, DieArea, InstanceType, Net,
    Netlist, Numeric, OrientType, ParsedInstance, ParsedPin, PlacementType, Size, VecCoords,
    VecInstances, VecSizes,
};

#[derive(Debug)]
pub struct DB<T: Numeric> {
    pub diearea: DieArea<T>,
    // orderder like: MOVABLE - FIXED - TERMINAL (FIXED)
    pub instances: VecInstances<T>,
    pub netlist: Netlist<T>,

    pub num_movable: usize,
    pub num_fixed: usize,
    // terminal are faked instance created for IO PINS (always fixed)
    pub num_terminal: usize,
    pub num_macro: usize,
    pub movable_area: T,
    pub fixed_area: T,
    pub cell_utilization: T,
}
impl<T: Numeric> DB<T> {
    fn new(diearea: DieArea<T>, instances: VecInstances<T>, netlist: Netlist<T>) -> Self {
        let mut movable_area = T::zero();
        let mut fixed_area = T::zero();

        let mut num_movable = 0;
        let mut num_fixed = 0;
        let mut num_terminal = 0;
        let mut num_macro = 0;

        for i in 0..instances.len() {
            if instances.is_movable(i) {
                num_movable += 1;
            } else {
                num_fixed += 1;
            }
            if instances.is_macro(i) {
                num_macro += 1;
            }
            if instances.is_terminal(i) {
                num_terminal += 1;
                // sanity check
                assert!(!instances.is_movable(i));
            }
        }

        assert_eq!(instances.len(), num_movable + num_fixed);

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
            num_fixed,
            num_terminal,
            num_macro,
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
        writeln!(f, "  MOVABLE AREA    : {:?}", self.movable_area)?;
        writeln!(f, "  FIXED AREA      : {:?}", self.fixed_area)?;
        writeln!(f, "  NUM MOVABLE     : {:?}", self.num_movable)?;
        writeln!(f, "  NUM FIXED       : {:?}", self.num_fixed)?;
        writeln!(f, "  NUM TERMINAL    : {:?}", self.num_terminal)?;
        writeln!(f, "  NUM MACRO       : {:?}", self.num_macro)?;
        writeln!(f, "  CELL UTILIZATION: {:?}", self.cell_utilization)?;

        Ok(())
    }
}

fn read_instances<'a, T: Numeric>(
    lef: &'a LEF,
    def: &'a DEF,
    _diearea: &DieArea<T>,
) -> Result<(Vec<ParsedInstance<'a, T>>, VecInstances<T>)> {
    let units = get_lef_def_units(lef, def)?;
    let scale = get_scale(lef)?;
    let lef_scale = get_lef_scale(lef)?;

    let name_to_lib: HashMap<&str, _> = lef
        .macros
        .iter()
        .flatten()
        .map(|m| (m.name.as_str(), m))
        .collect();

    let zero = T::zero();

    let mut parsed_instances = Vec::new();
    let mut fixed_parsed_instances = Vec::new();

    let mut instances_x = Vec::new();
    let mut instances_y = Vec::new();
    let mut instances_w = Vec::new();
    let mut instances_h = Vec::new();
    let mut instances_type = Vec::new();
    // all to be MOVABLE
    let mut instances_pl_type = Vec::new();
    let mut instances_o_type = Vec::new();
    let mut instances_area = Vec::new();
    let mut instances_num_pins = Vec::new();

    let mut fixed_instances_x = Vec::new();
    let mut fixed_instances_y = Vec::new();
    let mut fixed_instances_w = Vec::new();
    let mut fixed_instances_h = Vec::new();
    let mut fixed_instances_type = Vec::new();
    // all to be FIXED
    let mut fixed_instances_pl_type = Vec::new();
    let mut fixed_instances_o_type = Vec::new();
    let mut fixed_instances_area = Vec::new();
    let mut fixed_instances_num_pins = Vec::new();

    let offset = Coords::new(zero, zero);
    let mut id = 0;

    let comps = match def.components.as_ref() {
        Some(c) => c,
        None => return Ok((parsed_instances, VecInstances::default())),
    };
    for comp in comps {
        let name = comp.name.as_str();
        let model = comp.model.as_str();

        let comp_lib = name_to_lib
            .get(model)
            .ok_or_eyre(format!("INSTANCE MODEL NOT FOUND IN LEF {model}"))?;

        let size = comp_lib.opts.size.as_ref().ok_or_eyre("SIZE NOT FOUND")?;
        let size_width = T::from(size.width).unwrap();
        let size_height = T::from(size.height).unwrap();
        let new_size = Size::new(
            size_width * units * lef_scale,
            size_height * units * lef_scale,
        );

        let (coords, orient_type) = match comp.opts.placement.as_ref() {
            Some(p) => {
                // let (diearea_offset_x, diearea_offset_y) =
                //     if comp_lib.is_in_site(diearea.site_name.clone()) {
                //         (diearea.offset.x, diearea.offset.y)
                //     } else {
                //         (T::zero(), T::zero())
                //     };

                let coords = Coords::new(
                    // ((T::from(p.1.x).unwrap() - diearea_offset_x) * units) * scale,
                    // ((T::from(p.1.y).unwrap() - diearea_offset_y) * units) * scale,
                    ((T::from(p.1.x).unwrap()) * units) * scale,
                    ((T::from(p.1.y).unwrap()) * units) * scale,
                );

                let orient = OrientType::from(&p.2);

                assert!(
                    coords.x >= zero,
                    "negative x {:?} (originally {:?}) with offset {:?} for {}",
                    coords.x,
                    p.1.x,
                    // diearea_offset_x,
                    0,
                    name
                );
                assert!(
                    coords.y >= zero,
                    "negative y {:?} (originally {:?}) with offset {:?} for {}",
                    coords.y,
                    p.1.y,
                    // diearea_offset_y,
                    0,
                    name
                );

                (coords, orient)
            }
            None => (Coords::new(T::zero(), T::zero()), OrientType::default()),
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

        let num_pins = T::from(comp_lib.opts.pins.as_ref().map_or(0, |pins| pins.len())).unwrap();

        let push_instance = |xs: &mut Vec<T>,
                             ys: &mut Vec<T>,
                             ws: &mut Vec<T>,
                             hs: &mut Vec<T>,
                             types: &mut Vec<InstanceType>,
                             pl_types: &mut Vec<PlacementType>,
                             o_types: &mut Vec<OrientType>,
                             areas: &mut Vec<T>,
                             pins: &mut Vec<T>| {
            xs.push(coords.x);
            ys.push(coords.y);
            ws.push(new_size.width);
            hs.push(new_size.height);
            types.push(instance_type.clone());
            pl_types.push(placement_type.clone());
            o_types.push(orient_type.clone());
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
                    &mut fixed_instances_type,
                    &mut fixed_instances_pl_type,
                    &mut fixed_instances_o_type,
                    &mut fixed_instances_area,
                    &mut fixed_instances_num_pins,
                );

                fixed_parsed_instances.push(ParsedInstance::new(
                    0,
                    name,
                    model,
                    instance_type,
                    placement_type,
                    orient_type,
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
                    &mut instances_type,
                    &mut instances_pl_type,
                    &mut instances_o_type,
                    &mut instances_area,
                    &mut instances_num_pins,
                );

                parsed_instances.push(ParsedInstance::new(
                    id,
                    name,
                    model,
                    instance_type,
                    placement_type,
                    orient_type,
                    coords,
                    offset,
                    new_size,
                ));

                id += 1;
            }
        }
    }

    instances_x.extend(fixed_instances_x);
    instances_y.extend(fixed_instances_y);
    instances_w.extend(fixed_instances_w);
    instances_h.extend(fixed_instances_h);
    instances_type.extend(fixed_instances_type);
    instances_pl_type.extend(fixed_instances_pl_type);
    instances_o_type.extend(fixed_instances_o_type);
    instances_area.extend(fixed_instances_area);
    instances_num_pins.extend(fixed_instances_num_pins);

    // fixed instances are last ones
    for fixed in &mut fixed_parsed_instances {
        fixed.update_id(id);
        id += 1;
    }

    parsed_instances.extend(fixed_parsed_instances);

    let instances = VecInstances::new(
        VecCoords::new(instances_x, instances_y),
        VecSizes::new(instances_w, instances_h),
        instances_type,
        instances_pl_type,
        instances_o_type,
        instances_area,
        instances_num_pins,
    );

    assert_eq!(parsed_instances.len(), instances.len());

    Ok((parsed_instances, instances))
}

fn read_diearea<T: Numeric>(lef: &LEF, def: &DEF, full_diearea: bool) -> Result<DieArea<T>> {
    let scale = get_scale(lef)?;

    // site
    let site = lef.get_site();

    // TODO: offsets
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

    let units = get_lef_def_units(lef, def)?;

    let (offset_x, offset_y, diearea_width, diearea_height) = diearea.get_rectangle()?;

    let die_offset_x = site_offset_x + offset_x;
    let die_offset_y = site_offset_y + offset_y;
    let site_name = site.map(|s| s.name.clone());

    let mut diearea = DieArea::new(
        site_name,
        Size::new(
            (T::from(diearea_width - site_offset_x * 2.).unwrap() * units) * scale,
            (T::from(diearea_height - site_offset_y * 2.).unwrap() * units) * scale,
        ),
        Size::new(
            (T::from(diearea_width).unwrap() * units) * scale,
            (T::from(diearea_height).unwrap() * units) * scale,
        ),
        Coords::new(
            T::from(die_offset_x).unwrap(),
            T::from(die_offset_y).unwrap(),
        ),
    );

    if full_diearea {
        diearea.use_full_size();
    }

    Ok(diearea)
}

// in case of IO/PIN a new fake instance is created for each one of them
fn read_netlist<'a, T: Numeric>(
    lef: &LEF,
    def: &DEF,
    parsed_instances: &Vec<ParsedInstance<'a, T>>,
    instances: &mut VecInstances<T>,
) -> Result<Netlist<T>> {
    let units = get_lef_def_units(lef, def)?;
    let scale = get_scale(lef)?;
    let lef_scale = get_lef_scale(lef)?;

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

    let name_to_defpin: HashMap<&str, &DEFPin> = def
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
            parse_net(
                &name_to_defpin,
                &name_to_lib,
                &inst_name_to_cell,
                &net,
                units,
                scale,
                lef_scale,
                instances,
                &mut temp_2nets,
                &mut temp_3nets,
                &mut temp_nnets,
            )?;
        }
    }

    let mut pin_id: usize = 0;
    let mut parsed_pins = vec![];
    let mut pins_x = vec![];
    let mut pins_y = vec![];
    let mut pin_2_macro = vec![];
    let mut macro_2_pins = vec![vec![]; instances.len()];
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
    full_diearea: bool,
) -> Result<DB<T>> {
    let lef = read_lef(lef_path)?;
    let def = read_def(def_path)?;

    let diearea = read_diearea(&lef, &def, full_diearea)?;
    let (parsed_instances, mut instances) = read_instances(&lef, &def, &diearea)?;
    let netlist = read_netlist(&lef, &def, &parsed_instances, &mut instances)?;

    let db = DB::new(diearea, instances, netlist);

    log::info!("{}", lef);
    log::info!("{}", def);
    log::info!("{}", db);

    Ok(db)
}
