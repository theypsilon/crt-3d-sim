/* Copyright (c) 2019 José manuel Barroso Galindo <theypsilon@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>. */

use lazy_static::lazy_static;

use crate::field_changer::FieldChanger;
use crate::simulation_core_ticker::SimulationUpdater;

pub trait ViewController {
    fn id(&self) -> &'static str;
    fn keys_inc(&self) -> &[&'static str];
    fn keys_dec(&self) -> &[&'static str];
    fn update(&self, updater: &mut SimulationUpdater) -> bool;
}

pub struct ColorNoise {}

impl ViewController for ColorNoise {
    fn id(&self) -> &'static str {
        return "color-noise";
    }
    fn keys_inc(&self) -> &[&'static str] {
        &[]
    }
    fn keys_dec(&self) -> &[&'static str] {
        &[]
    }
    fn update(&self, updater: &mut SimulationUpdater) -> bool {
        let filters = &mut updater.res.filters;
        let ctx = &updater.ctx;
        let input = &updater.input;
        FieldChanger::new(*ctx, &mut filters.color_noise, input.color_noise)
            .set_progression(0.01 * updater.dt * updater.res.speed.filter_speed)
            .set_event_value(input.event_color_noise)
            .set_min(0.0)
            .set_max(1.0)
            .set_trigger_handler(|x| ctx.dispatcher().dispatch_color_noise(x))
            .process_with_sums()
    }
}

lazy_static! {
    static ref VIEW_OPTIONS: [Box<dyn ViewController + Sync>; 1] = [Box::new(ColorNoise {})];
}
