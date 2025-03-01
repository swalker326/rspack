//!  There are methods whose verb is `ChunkGraphChunk`

use indexmap::IndexSet;
use rspack_database::Database;
use rspack_identifier::{IdentifierLinkedMap, IdentifierSet};
use rustc_hash::FxHashMap as HashMap;

use crate::{
  find_graph_roots, merge_runtime, BoxModule, Chunk, ChunkByUkey, ChunkGroup, ChunkGroupByUkey,
  ChunkGroupUkey, ChunkUkey, Module, ModuleGraph, ModuleGraphModule, ModuleIdentifier,
  RuntimeGlobals, SourceType,
};
use crate::{ChunkGraph, Compilation};

#[derive(Debug, Clone, Default)]
pub struct ChunkSizeOptions {
  // constant overhead for a chunk
  pub chunk_overhead: Option<f64>,
  // multiplicator for initial chunks
  pub entry_chunk_multiplicator: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ChunkGraphChunk {
  /// URI of modules => ChunkGroupUkey
  ///
  /// use `LinkedHashMap` to keep the ordered from entry array.
  pub(crate) entry_modules: IdentifierLinkedMap<ChunkGroupUkey>,
  pub modules: IdentifierSet,
  pub runtime_requirements: RuntimeGlobals,
  pub(crate) runtime_modules: Vec<ModuleIdentifier>,
}

impl ChunkGraphChunk {
  pub fn new() -> Self {
    Self {
      entry_modules: Default::default(),
      modules: Default::default(),
      runtime_requirements: Default::default(),
      runtime_modules: Default::default(),
    }
  }
}

fn get_modules_size(modules: &[&BoxModule]) -> f64 {
  let mut size = 0f64;
  for module in modules {
    for source_type in module.source_types() {
      size += module.size(source_type);
    }
  }
  size
}

impl ChunkGraph {
  pub fn add_chunk(&mut self, chunk_ukey: ChunkUkey) {
    self
      .chunk_graph_chunk_by_chunk_ukey
      .entry(chunk_ukey)
      .or_default();
  }
  pub fn add_chunk_wit_chunk_graph_chunk(&mut self, chunk_ukey: ChunkUkey, cgc: ChunkGraphChunk) {
    debug_assert!(!self
      .chunk_graph_chunk_by_chunk_ukey
      .contains_key(&chunk_ukey));
    self.chunk_graph_chunk_by_chunk_ukey.insert(chunk_ukey, cgc);
  }

  pub fn get_chunk_entry_modules(&self, chunk_ukey: &ChunkUkey) -> Vec<ModuleIdentifier> {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk_ukey);

    chunk_graph_chunk.entry_modules.keys().cloned().collect()
  }

  pub fn get_chunk_entry_modules_with_chunk_group_iterable(
    &self,
    chunk_ukey: &ChunkUkey,
  ) -> &IdentifierLinkedMap<ChunkGroupUkey> {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    &cgc.entry_modules
  }

  pub fn get_chunk_graph_chunk_mut(&mut self, chunk_ukey: ChunkUkey) -> &mut ChunkGraphChunk {
    self
      .chunk_graph_chunk_by_chunk_ukey
      .get_mut(&chunk_ukey)
      .expect("Chunk should be added before")
  }

  pub fn get_chunk_graph_chunk(&self, chunk_ukey: &ChunkUkey) -> &ChunkGraphChunk {
    self
      .chunk_graph_chunk_by_chunk_ukey
      .get(chunk_ukey)
      .expect("Chunk should be added before")
  }

  pub fn connect_chunk_and_entry_module(
    &mut self,
    chunk: ChunkUkey,
    module_identifier: ModuleIdentifier,
    entrypoint: ChunkGroupUkey,
  ) {
    let chunk_graph_module = self
      .chunk_graph_module_by_module_identifier
      .entry(module_identifier)
      .or_default();
    chunk_graph_module.entry_in_chunks.insert(chunk);

    let chunk_graph_chunk = self
      .chunk_graph_chunk_by_chunk_ukey
      .entry(chunk)
      .or_default();
    chunk_graph_chunk
      .entry_modules
      .insert(module_identifier, entrypoint);
  }

  pub fn disconnect_chunk_and_module(
    &mut self,
    chunk: &ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    let chunk_graph_module = self.get_chunk_graph_module_mut(module_identifier);
    chunk_graph_module.chunks.remove(chunk);

    let chunk_graph_chunk = self.get_chunk_graph_chunk_mut(*chunk);
    chunk_graph_chunk.modules.remove(&module_identifier);
  }

  pub fn connect_chunk_and_module(
    &mut self,
    chunk: ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    let chunk_graph_module = self
      .chunk_graph_module_by_module_identifier
      .entry(module_identifier)
      .or_default();
    chunk_graph_module.chunks.insert(chunk);

    let chunk_graph_chunk = self
      .chunk_graph_chunk_by_chunk_ukey
      .entry(chunk)
      .or_default();
    chunk_graph_chunk.modules.insert(module_identifier);
  }

  pub fn connect_chunk_and_runtime_module(
    &mut self,
    chunk: ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    self
      .chunk_graph_module_by_module_identifier
      .entry(module_identifier)
      .or_default();
    let cgm = self.get_chunk_graph_module_mut(module_identifier);
    cgm.runtime_in_chunks.insert(chunk);

    self
      .chunk_graph_chunk_by_chunk_ukey
      .entry(chunk)
      .or_default();
    let cgc = self.get_chunk_graph_chunk_mut(chunk);
    if !cgc.runtime_modules.contains(&module_identifier) {
      cgc.runtime_modules.push(module_identifier);
    }
  }

  pub fn get_chunk_modules<'module>(
    &self,
    chunk: &ChunkUkey,
    module_graph: &'module ModuleGraph,
  ) -> Vec<&'module BoxModule> {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    chunk_graph_chunk
      .modules
      .iter()
      .filter_map(|uri| module_graph.module_by_identifier(uri))
      .collect()
  }

  pub fn get_chunk_module_identifiers(&self, chunk: &ChunkUkey) -> &IdentifierSet {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    &chunk_graph_chunk.modules
  }

  pub fn get_ordered_chunk_modules<'module>(
    &self,
    chunk: &ChunkUkey,
    module_graph: &'module ModuleGraph,
  ) -> Vec<&'module BoxModule> {
    let mut modules = self.get_chunk_modules(chunk, module_graph);
    // SAFETY: module identifier is unique
    modules.sort_unstable_by_key(|m| m.identifier().as_str());
    modules
  }

  pub fn get_chunk_modules_by_source_type<'module>(
    &self,
    chunk: &ChunkUkey,
    source_type: SourceType,
    module_graph: &'module ModuleGraph,
  ) -> Vec<&'module ModuleGraphModule> {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    let modules = chunk_graph_chunk
      .modules
      .iter()
      .filter_map(|uri| module_graph.module_graph_module_by_identifier(uri))
      .filter(|mgm| {
        module_graph
          .module_by_identifier(&mgm.module_identifier)
          .map(|module| module.source_types().contains(&source_type))
          .unwrap_or_default()
      })
      .collect::<Vec<_>>();
    modules
  }

  pub fn get_chunk_modules_iterable_by_source_type<'module_graph: 'me, 'me>(
    &'me self,
    chunk: &ChunkUkey,
    source_type: SourceType,
    module_graph: &'module_graph ModuleGraph,
  ) -> impl Iterator<Item = &'module_graph dyn Module> + 'me {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    chunk_graph_chunk
      .modules
      .iter()
      .filter_map(|uri| module_graph.module_by_identifier(uri))
      .filter(move |module| module.source_types().contains(&source_type))
      .map(|m| m.as_ref())
  }

  pub fn get_chunk_modules_size(&self, chunk: &ChunkUkey, module_graph: &ModuleGraph) -> f64 {
    self
      .get_chunk_modules(chunk, module_graph)
      .iter()
      .fold(0.0, |acc, m| {
        acc + m.source_types().iter().fold(0.0, |acc, t| acc + m.size(t))
      })
  }

  pub fn get_number_of_chunk_modules(&self, chunk: &ChunkUkey) -> usize {
    let cgc = self.get_chunk_graph_chunk(chunk);
    cgc.modules.len()
  }

  pub fn get_number_of_entry_modules(&self, chunk: &ChunkUkey) -> usize {
    let cgc = self.get_chunk_graph_chunk(chunk);
    cgc.entry_modules.len()
  }

  pub fn add_chunk_runtime_requirements(
    &mut self,
    chunk_ukey: &ChunkUkey,
    runtime_requirements: RuntimeGlobals,
  ) {
    let cgc = self.get_chunk_graph_chunk_mut(*chunk_ukey);
    cgc.runtime_requirements.insert(runtime_requirements);
  }

  pub fn add_tree_runtime_requirements(
    &mut self,
    chunk_ukey: &ChunkUkey,
    runtime_requirements: RuntimeGlobals,
  ) {
    self.add_chunk_runtime_requirements(chunk_ukey, runtime_requirements);
  }

  pub fn get_chunk_runtime_requirements(&self, chunk_ukey: &ChunkUkey) -> &RuntimeGlobals {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    &cgc.runtime_requirements
  }

  pub fn get_tree_runtime_requirements(&self, chunk_ukey: &ChunkUkey) -> &RuntimeGlobals {
    self.get_chunk_runtime_requirements(chunk_ukey)
  }

  pub fn get_chunk_runtime_modules_in_order(
    &self,
    chunk_ukey: &ChunkUkey,
  ) -> &Vec<ModuleIdentifier> {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    &cgc.runtime_modules
  }

  pub fn get_chunk_runtime_modules_iterable(
    &self,
    chunk_ukey: &ChunkUkey,
  ) -> impl Iterator<Item = &ModuleIdentifier> {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    cgc.runtime_modules.iter()
  }

  pub fn get_chunk_condition_map<F: Fn(&ChunkUkey, &Compilation) -> bool>(
    &self,
    chunk_ukey: &ChunkUkey,
    compilation: &Compilation,
    filter: F,
  ) -> HashMap<String, bool> {
    let mut map = HashMap::default();

    let chunk = compilation
      .chunk_by_ukey
      .get(chunk_ukey)
      .expect("Chunk should exist");
    for c in chunk
      .get_all_referenced_chunks(&compilation.chunk_group_by_ukey)
      .iter()
    {
      let chunk = compilation
        .chunk_by_ukey
        .get(c)
        .expect("Chunk should exist");
      map.insert(chunk.expect_id().to_string(), filter(c, compilation));
    }

    map
  }

  pub fn get_chunk_root_modules(
    &self,
    chunk: &ChunkUkey,
    module_graph: &ModuleGraph,
  ) -> Vec<ModuleIdentifier> {
    let cgc = self.get_chunk_graph_chunk(chunk);
    let mut input = cgc.modules.iter().cloned().collect::<Vec<_>>();
    input.sort_unstable();
    let mut modules = find_graph_roots(input, |module| {
      let mut set: IdentifierSet = Default::default();
      fn add_dependencies(
        module: ModuleIdentifier,
        set: &mut IdentifierSet,
        module_graph: &ModuleGraph,
      ) {
        let module = module_graph
          .module_by_identifier(&module)
          .expect("should exist");
        for connection in module_graph.get_outgoing_connections(module) {
          // https://github.com/webpack/webpack/blob/1f99ad6367f2b8a6ef17cce0e058f7a67fb7db18/lib/ChunkGraph.js#L290
          let active_state = connection.get_active_state(module_graph, None);
          match active_state {
            crate::ConnectionState::Bool(false) => {
              continue;
            }
            crate::ConnectionState::TransitiveOnly => {
              add_dependencies(connection.module_identifier, set, module_graph);
              continue;
            }
            _ => {}
          }
          set.insert(connection.module_identifier);
        }
      }

      add_dependencies(module, &mut set, module_graph);
      set.into_iter().collect()
    });

    modules.sort_unstable();

    modules
  }
  pub fn disconnect_chunk(
    &mut self,
    chunk: &mut Chunk,
    chunk_group_by_ukey: &mut ChunkGroupByUkey,
  ) {
    let chunk_ukey = &chunk.ukey;
    let cgc = self.get_chunk_graph_chunk_mut(*chunk_ukey);
    let cgc_modules = std::mem::take(&mut cgc.modules);
    for module in cgc_modules {
      let cgm = self.get_chunk_graph_module_mut(module);
      cgm.chunks.remove(chunk_ukey);
    }
    chunk.disconnect_from_groups(chunk_group_by_ukey)
  }

  pub fn has_chunk_entry_dependent_chunks(
    &self,
    chunk_ukey: &ChunkUkey,
    chunk_group_by_ukey: &ChunkGroupByUkey,
  ) -> bool {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    for (_, chunk_group_ukey) in cgc.entry_modules.iter() {
      let chunk_group = chunk_group_by_ukey
        .get(chunk_group_ukey)
        .expect("should have chunk group");
      for c in chunk_group.chunks.iter() {
        if c != chunk_ukey {
          return true;
        }
      }
    }
    false
  }

  pub fn get_chunk_entry_dependent_chunks_iterable(
    &self,
    chunk_ukey: &ChunkUkey,
    chunk_by_ukey: &ChunkByUkey,
    chunk_group_by_ukey: &ChunkGroupByUkey,
  ) -> impl Iterator<Item = ChunkUkey> {
    let chunk = chunk_by_ukey.get(chunk_ukey).expect("should have chunk");
    let mut set = IndexSet::new();
    for chunk_group_ukey in chunk.get_sorted_groups_iter(chunk_group_by_ukey) {
      let chunk_group = chunk_group_by_ukey
        .get(chunk_group_ukey)
        .expect("should have chunk group");
      if chunk_group.kind.is_entrypoint() {
        let entry_point_chunk = chunk_group.get_entry_point_chunk();
        let cgc = self.get_chunk_graph_chunk(&entry_point_chunk);
        for (_, chunk_group_ukey) in cgc.entry_modules.iter() {
          let chunk_group = chunk_group_by_ukey
            .get(chunk_group_ukey)
            .expect("should have chunk group");
          for c in chunk_group.chunks.iter() {
            let chunk = chunk_by_ukey.get(c).expect("should have chunk");
            if c != chunk_ukey && c != &entry_point_chunk && !chunk.has_runtime(chunk_group_by_ukey)
            {
              set.insert(*c);
            }
          }
        }
      }
    }
    set.into_iter()
  }

  pub fn disconnect_chunk_and_entry_module(
    &mut self,
    chunk: &ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    let chunk_graph_module = self.get_chunk_graph_module_mut(module_identifier);
    chunk_graph_module.chunks.remove(chunk);

    let chunk_graph_chunk = self.get_chunk_graph_chunk_mut(*chunk);
    chunk_graph_chunk.entry_modules.remove(&module_identifier);
  }

  pub fn can_chunks_be_integrated(
    &self,
    chunk_a_ukey: &ChunkUkey,
    chunk_b_ukey: &ChunkUkey,
    chunk_by_ukey: &ChunkByUkey,
    chunk_group_by_ukey: &ChunkGroupByUkey,
  ) -> bool {
    let chunk_a = chunk_by_ukey.get(chunk_a_ukey).expect("should have chunk");
    let chunk_b = chunk_by_ukey.get(chunk_b_ukey).expect("should have chunk");
    if chunk_a.prevent_integration || chunk_b.prevent_integration {
      return false;
    }

    let has_runtime_a = chunk_a.has_runtime(chunk_group_by_ukey);
    let has_runtime_b = chunk_b.has_runtime(chunk_group_by_ukey);

    // true, if a is always a parent of b
    let is_available_chunk = |a: &Chunk, b: &Chunk| {
      let mut queue = b.groups.clone().into_iter().collect::<Vec<_>>();
      while let Some(chunk_group_ukey) = queue.pop() {
        if a.is_in_group(&chunk_group_ukey) {
          continue;
        }
        let chunk_group = chunk_group_by_ukey.expect_get(&chunk_group_ukey);
        if chunk_group.is_initial() {
          return false;
        }
        for parent in chunk_group.parents_iterable() {
          queue.push(*parent);
        }
      }
      true
    };

    if has_runtime_a != has_runtime_b {
      if has_runtime_a {
        return is_available_chunk(chunk_a, chunk_b);
      } else if has_runtime_b {
        return is_available_chunk(chunk_b, chunk_a);
      } else {
        return false;
      }
    }

    if self.get_number_of_entry_modules(&chunk_a.ukey) > 0
      || self.get_number_of_entry_modules(&chunk_b.ukey) > 0
    {
      return false;
    }

    true
  }

  pub fn get_chunk_size(
    &self,
    chunk_ukey: &ChunkUkey,
    options: &ChunkSizeOptions,
    chunk_by_ukey: &Database<Chunk>,
    chunk_group_by_ukey: &Database<ChunkGroup>,
    module_graph: &ModuleGraph,
  ) -> f64 {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    let modules: Vec<&BoxModule> = cgc
      .modules
      .iter()
      .filter_map(|id| module_graph.module_by_identifier(id))
      .collect::<Vec<_>>();
    let modules_size = get_modules_size(&modules);
    let chunk_overhead = options.chunk_overhead.unwrap_or(10000f64);
    let entry_chunk_multiplicator = options.entry_chunk_multiplicator.unwrap_or(10f64);
    let chunk = chunk_by_ukey.get(chunk_ukey).expect("chunk not found");
    chunk_overhead
      + modules_size
        * (if chunk.can_be_initial(chunk_group_by_ukey) {
          entry_chunk_multiplicator
        } else {
          1f64
        })
  }

  pub fn get_integrated_chunks_size(
    &self,
    chunk_a_ukey: &ChunkUkey,
    chunk_b_ukey: &ChunkUkey,
    options: &ChunkSizeOptions,
    chunk_by_ukey: &Database<Chunk>,
    chunk_group_by_ukey: &Database<ChunkGroup>,
    module_graph: &ModuleGraph,
  ) -> f64 {
    let cgc_a = self.get_chunk_graph_chunk(chunk_a_ukey);
    let cgc_b = self.get_chunk_graph_chunk(chunk_b_ukey);
    let mut all_modules: Vec<&BoxModule> = cgc_a
      .modules
      .iter()
      .filter_map(|id| module_graph.module_by_identifier(id))
      .collect::<Vec<_>>();
    for id in &cgc_b.modules {
      let module = module_graph.module_by_identifier(id);
      if let Some(module) = module {
        all_modules.push(module);
      }
    }
    let modules_size = get_modules_size(&all_modules);
    let chunk_overhead = options.chunk_overhead.unwrap_or(10000f64);
    let entry_chunk_multiplicator = options.entry_chunk_multiplicator.unwrap_or(10f64);

    let chunk_a = chunk_by_ukey.get(chunk_a_ukey).expect("chunk not found");
    let chunk_b = chunk_by_ukey.get(chunk_b_ukey).expect("chunk not found");

    chunk_overhead
      + modules_size
        * (if chunk_a.can_be_initial(chunk_group_by_ukey)
          || chunk_b.can_be_initial(chunk_group_by_ukey)
        {
          entry_chunk_multiplicator
        } else {
          1f64
        })
  }

  pub fn integrate_chunks(
    &mut self,
    a: &ChunkUkey,
    b: &ChunkUkey,
    chunk_by_ukey: &mut Database<Chunk>,
    chunk_group_by_ukey: &mut Database<ChunkGroup>,
    module_graph: &ModuleGraph,
  ) {
    let chunk_b = chunk_by_ukey.expect_get(b).clone();
    let chunk_a = chunk_by_ukey.expect_get_mut(a);

    // Decide for one name (deterministic)
    if let (Some(_), Some(_)) = (&chunk_a.name, &chunk_b.name) {
      if (self.get_number_of_entry_modules(a) > 0) == (self.get_number_of_entry_modules(b) > 0) {
        // When both chunks have entry modules or none have one, use
        // shortest name
        match (chunk_a.name.clone(), chunk_b.name.clone()) {
          (Some(a), Some(b)) => {
            if a.len() != b.len() {
              chunk_a.name = if a.len() < b.len() { Some(a) } else { Some(b) };
            }
          }
          (None, Some(b)) => {
            chunk_a.name = Some(b);
          }
          _ => {}
        }
      } else if self.get_number_of_entry_modules(b) > 0 {
        // Pick the name of the chunk with the entry module
        chunk_a.name = chunk_b.name.clone();
      }
    } else if chunk_b.name.is_some() {
      chunk_a.name = chunk_b.name.clone();
    }

    // Merge id name hints
    for hint in &chunk_b.id_name_hints {
      chunk_a.id_name_hints.insert(hint.clone());
    }

    // Merge runtime
    chunk_a.runtime = merge_runtime(&chunk_a.runtime, &chunk_b.runtime);

    // get_chunk_modules is used here to create a clone, because disconnect_chunk_and_module modifies
    for module in self.get_chunk_modules(b, module_graph) {
      self.disconnect_chunk_and_module(b, module.identifier());
      self.connect_chunk_and_module(*a, module.identifier());
    }

    for (module, chunk_group) in self
      .clone()
      .get_chunk_entry_modules_with_chunk_group_iterable(b)
      .iter()
    {
      self.disconnect_chunk_and_entry_module(b, *module);
      self.connect_chunk_and_entry_module(*a, *module, *chunk_group);
    }

    let mut remove_group_ukeys = vec![];
    for chunk_group_ukey in chunk_b.groups {
      let chunk_group = chunk_group_by_ukey.expect_get_mut(&chunk_group_ukey);
      chunk_group.replace_chunk(b, a);
      chunk_a.add_group(chunk_group_ukey);
      remove_group_ukeys.push(chunk_group_ukey);
    }

    let chunk_b = chunk_by_ukey.expect_get_mut(b);
    for group_ukey in remove_group_ukeys {
      chunk_b.remove_group(&group_ukey);
    }
  }
}
