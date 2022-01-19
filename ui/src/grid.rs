// Copyright 2018 The Synthesizer IO Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Datatypes representing the model of a patching grid.
use druid::im::{HashSet, Vector};
use druid::Data;

#[derive(Default, Data, Clone)]
pub struct WireGrid {
    grid: HashSet<(u16, u16, bool)>,
    jumpers: Vector<(u16, u16, u16, u16)>,
}

#[derive(Default, Data, Clone)]
pub struct ModuleGrid {
    modules: Vector<ModuleInstance>,
}

#[derive(Data, Clone, Debug, PartialEq, Eq)]
pub struct ModuleInstance {
    pub loc: (u16, u16),
    pub spec: ModuleSpec,
}

#[derive(Data, Clone, Debug, PartialEq, Eq)]
pub struct ModuleSpec {
    pub size: (u16, u16),
    pub name: String,
}

#[derive(Data, Clone, Debug)]
pub enum Delta {
    Wire(WireDelta),
    Jumper(JumperDelta),
    /// Add a module. Note: we need to encode moving and deleting as well, and
    /// probably have a unique id mechanism. Later.
    Module(ModuleInstance),
}

#[derive(Data, Clone, Debug)]
pub struct WireDelta {
    pub grid_ix: (u16, u16, bool),
    pub val: bool,
}

#[derive(Data, Clone, Debug)]
pub struct JumperDelta {
    pub start: (u16, u16),
    pub end: (u16, u16),
    // true for add, false for delete
    pub val: bool,
}

impl WireGrid {
    pub fn set(&mut self, grid_ix: (u16, u16, bool), val: bool) {
        if val {
            self.grid.insert(grid_ix);
        } else {
            self.grid.remove(&grid_ix);
        }
    }

    pub fn is_set(&self, grid_ix: (u16, u16, bool)) -> bool {
        self.grid.contains(&grid_ix)
    }

    pub fn unit_line_to_grid_ix(x0: u16, y0: u16, x1: u16, y1: u16) -> (u16, u16, bool) {
        if x1 == x0 + 1 {
            (x0, y0, false)
        } else if x0 == x1 + 1 {
            (x1, y0, false)
        } else if y1 == y0 + 1 {
            (x0, y0, true)
        } else if y0 == y1 + 1 {
            (x0, y1, true)
        } else {
            panic!("not a unit line, logic error");
        }
    }

    pub fn apply_jumper_delta(&mut self, delta: JumperDelta) {
        //println!("apply jumper {:?}", delta);
        let coords = (delta.start.0, delta.start.1, delta.end.0, delta.end.1);
        if delta.val {
            self.jumpers.push_back(coords);
        } else if let Some(pos) = self.jumpers.iter().position(|&c| c == coords) {
            self.jumpers.remove(pos);
        } else {
            println!("trying to delete nonexistent jumper");
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(u16, u16, bool)> {
        self.grid.iter()
    }

    pub fn iter_jumpers(&self) -> impl Iterator<Item = &(u16, u16, u16, u16)> {
        self.jumpers.iter()
    }
}

impl ModuleInstance {
    /// Determine whether this instance conflicts with another proposed instance.
    fn is_conflict(&self, other: &ModuleInstance) -> bool {
        self.loc.0 + self.spec.size.0 >= other.loc.0
            && other.loc.0 + other.spec.size.0 >= self.loc.0
            && self.loc.1 + self.spec.size.1 >= other.loc.1
            && other.loc.1 + other.spec.size.1 >= self.loc.1
    }
}

impl ModuleGrid {
    /// Add a module instance to the grid.
    pub fn add(&mut self, instance: ModuleInstance) {
        self.modules.push_back(instance);
    }

    /// Iterate through the instances on the grid.
    pub fn iter(&self) -> impl Iterator<Item = &ModuleInstance> {
        self.modules.iter()
    }

    /// Determine whether the proposed instance conflict with any on the grid.
    pub fn is_conflict(&self, other: &ModuleInstance) -> bool {
        self.iter().any(|inst| inst.is_conflict(other))
    }

    /// Determine the output pin of a module. Needs rework for multi-output modules.
    pub fn determine_output_pin(inst: &ModuleInstance) -> (u16, u16) {
        (inst.loc.0 + 2, inst.loc.1)
    }
}
