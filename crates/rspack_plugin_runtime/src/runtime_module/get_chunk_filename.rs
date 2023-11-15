use std::cmp::Ordering;
use std::collections::HashMap as RawHashMap;
use std::collections::HashSet as RawHashSet;

use itertools::Itertools;
use rspack_core::ChunkKind;
use rspack_core::{
  get_css_chunk_filename_template, get_js_chunk_filename_template,
  rspack_sources::{BoxSource, RawSource, SourceExt},
  stringify_map, Chunk, ChunkUkey, Compilation, Filename, PathData, RuntimeGlobals, RuntimeModule,
  SourceType,
};
use rspack_identifier::Identifier;
use rustc_hash::FxHashMap as HashMap;

use super::utils::chunk_has_css;
use crate::impl_runtime_module;
use crate::runtime_module::create_chunk_filename_template;
use crate::runtime_module::unquoted_stringify;

#[derive(Debug, Eq)]
pub struct GetChunkFilenameRuntimeModule {
  id: Identifier,
  chunk: Option<ChunkUkey>,
  content_type: &'static str,
  source_type: SourceType,
  global: RuntimeGlobals,
  all_chunks: bool,
}
// It's render is different with webpack, rspack will only render chunk map<chunkId, chunkName>
// and search it.
impl GetChunkFilenameRuntimeModule {
  pub fn new(
    content_type: &'static str,
    source_type: SourceType,
    global: RuntimeGlobals,
    all_chunks: bool,
  ) -> Self {
    Self {
      id: Identifier::from(format!("webpack/runtime/get_chunk_filename/{content_type}")),
      chunk: None,
      content_type,
      source_type,
      global,
      all_chunks,
    }
  }
}

impl RuntimeModule for GetChunkFilenameRuntimeModule {
  fn name(&self) -> Identifier {
    self.id
  }

  fn cacheable(&self) -> bool {
    false
  }

  fn generate(&self, compilation: &Compilation) -> BoxSource {
    let chunks = if let Some(chunk) = self.chunk {
      if let Some(chunk) = compilation.chunk_by_ukey.get(&chunk) {
        if self.all_chunks {
          Some(chunk.get_all_referenced_chunks(&compilation.chunk_group_by_ukey))
        } else {
          let mut chunks = chunk.get_all_async_chunks(&compilation.chunk_group_by_ukey);
          if compilation
            .chunk_graph
            .get_tree_runtime_requirements(&chunk.ukey)
            .contains(RuntimeGlobals::ENSURE_CHUNK_INCLUDE_ENTRIES)
          {
            for c in compilation
              .chunk_graph
              .get_chunk_entry_dependent_chunks_iterable(
                &chunk.ukey,
                &compilation.chunk_by_ukey,
                &compilation.chunk_group_by_ukey,
              )
            {
              chunks.insert(c);
            }
          }
          Some(chunks)
        }
      } else {
        None
      }
    } else {
      None
    };

    let mut dynamic_filename: Option<&Filename> = None;
    let mut max_chunk_set_size = 0;
    let mut chunk_filenames = RawHashMap::new();
    let mut chunk_map = RawHashMap::new();

    let mut dynamic_chunk_url = None;
    let mut static_chunk_url = RawHashMap::new();

    if let Some(chunks) = chunks {
      chunks.iter().for_each(|chunk_ukey| {
        if let Some(chunk) = compilation.chunk_by_ukey.get(chunk_ukey) {
          let filename_template = match self.source_type {
            // TODO webpack different
            // css chunk will generate a js chunk, so here add it.
            SourceType::JavaScript => Some(get_js_chunk_filename_template(
              chunk,
              &compilation.options.output,
              &compilation.chunk_group_by_ukey,
            )),
            SourceType::Css => {
              if chunk_has_css(chunk_ukey, compilation) {
                Some(get_css_chunk_filename_template(
                  chunk,
                  &compilation.options.output,
                  &compilation.chunk_group_by_ukey,
                ))
              } else {
                None
              }
            }
            _ => unreachable!(),
          };

          if let Some(filename_template) = filename_template {
            chunk_map.insert(chunk_ukey, (chunk, filename_template));

            let chunk_set = chunk_filenames
              .entry(filename_template.clone())
              .or_insert(RawHashSet::new());

            chunk_set.insert(chunk_ukey);

            let should_update = match dynamic_filename {
              Some(dynamic_filename) => {
                let chunk_set_size = chunk_set.len();
                let filename_len = filename_template.template().len();
                let dynamic_len = dynamic_filename.template().len();

                match chunk_set_size.cmp(&max_chunk_set_size) {
                  Ordering::Less => false,
                  Ordering::Greater => true,
                  Ordering::Equal => match filename_len.cmp(&dynamic_len) {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => !matches!(
                      filename_template
                        .template()
                        .cmp(dynamic_filename.template()),
                      Ordering::Less,
                    ),
                  },
                }
              }
              None => true,
            };
            if should_update {
              max_chunk_set_size = chunk_set.len();
              dynamic_filename = Some(filename_template);
            }
          }
        }
      });

      fn create_dynamic_chunk_content_map<F>(
        f: F,
        chunks: &RawHashSet<&ChunkUkey>,
        chunk_map: &RawHashMap<&ChunkUkey, (&Chunk, &Filename)>,
      ) -> String
      where
        F: Fn(&Chunk) -> Option<String>,
      {
        let mut result = HashMap::default();
        let mut use_id = false;
        let mut last_key = None;
        let mut entries = 0;

        for chunk_ukey in chunks.iter() {
          if let Some((chunk, _)) = chunk_map.get(chunk_ukey) {
            if let Some(chunk_id) = &chunk.id {
              if let Some(value) = f(chunk) {
                if value == *chunk_id {
                  use_id = true;
                } else {
                  result.insert(
                    chunk_id.clone(),
                    serde_json::to_string(&value).expect("invalid json to_string"),
                  );
                  last_key = Some(chunk_id.clone());
                  entries += 1;
                }
              }
            }
          }
        }

        let content = if entries == 0 {
          "chunkId".to_string()
        } else if entries == 1 {
          if let Some(last_key) = last_key {
            if use_id {
              format!(
                "(chunkId === {} ? {} : chunkId)`",
                serde_json::to_string(&last_key).expect("invalid json to_string"),
                result.get(&last_key).expect("cannot find last key value")
              )
            } else {
              result
                .get(&last_key)
                .expect("cannot find last key value")
                .clone()
            }
          } else {
            unreachable!();
          }
        } else if use_id {
          format!("({}[chunkId] || chunkId)", stringify_map(&result))
        } else {
          format!("{}[chunkId]", stringify_map(&result))
        };
        format!("\" + {content} + \"")
      }

      for (filename, chunks) in chunk_filenames.iter() {
        if let Some(dynamic_filename) = dynamic_filename {
          if filename == dynamic_filename {
            let (fake_filename, hash_len_map) = create_chunk_filename_template(filename);

            let mut fake_chunk = Chunk::new(None, ChunkKind::Normal);
            fake_chunk.name = Some(create_dynamic_chunk_content_map(
              |c| match &c.name {
                Some(name) => Some(name.to_string()),
                None => c.id.clone().map(|id| id.to_string()),
              },
              chunks,
              &chunk_map,
            ));
            fake_chunk.rendered_hash = Some(
              create_dynamic_chunk_content_map(
                |c| {
                  let hash = c.rendered_hash.as_ref().map(|hash| hash.to_string());
                  match hash_len_map.get("[chunkhash]") {
                    Some(hash_len) => hash.map(|s| s[..*hash_len].to_string()),
                    None => hash,
                  }
                },
                chunks,
                &chunk_map,
              )
              .into(),
            );
            fake_chunk.id = Some("\" + chunkId + \"".to_string());

            let content_hash = Some(create_dynamic_chunk_content_map(
              |c| {
                c.content_hash.get(&self.source_type).map(|i| {
                  let hash = i
                    .rendered(compilation.options.output.hash_digest_length)
                    .to_string();
                  match hash_len_map.get("[contenthash]") {
                    Some(hash_len) => hash[..*hash_len].to_string(),
                    None => hash,
                  }
                })
              },
              chunks,
              &chunk_map,
            ));

            let hash = match hash_len_map
              .get("[fullhash]")
              .or(hash_len_map.get("[hash]"))
            {
              Some(hash_len) => format!(
                "\" + {}().slice(0, {}) + \"",
                RuntimeGlobals::GET_FULL_HASH,
                hash_len
              ),
              None => format!("\" + {}() + \"", RuntimeGlobals::GET_FULL_HASH),
            };

            dynamic_chunk_url = Some(format!(
              "\"{}\"",
              compilation.get_path(
                &fake_filename,
                PathData::default()
                  .chunk(&fake_chunk)
                  .hash_optional(Some(hash.as_str()))
                  .content_hash_optional(content_hash.as_deref()),
              )
            ));
          }
        } else {
          for chunk_ukey in chunks.iter() {
            if let Some((chunk, filename_template)) = chunk_map.get(chunk_ukey) {
              let (fake_filename, hash_len_map) = create_chunk_filename_template(filename_template);

              let mut fake_chunk = Chunk::new(None, ChunkKind::Normal);
              fake_chunk.id = chunk
                .id
                .as_ref()
                .map(|chunk_id| unquoted_stringify(chunk, chunk_id));

              fake_chunk.rendered_hash = chunk.rendered_hash.as_ref().map(|chunk_hash| {
                let hash = unquoted_stringify(chunk, &chunk_hash.as_ref().to_string());
                let res = match hash_len_map.get("[chunkhash]") {
                  Some(hash_len) => hash[..*hash_len].to_string(),
                  None => hash,
                };
                res.into()
              });

              if let Some(chunk_name) = &chunk.name {
                fake_chunk.name = Some(unquoted_stringify(chunk, chunk_name));
              } else if let Some(chunk_id) = &chunk.id {
                fake_chunk.name = Some(unquoted_stringify(chunk, chunk_id));
              }

              let content_hash = chunk.content_hash.get(&self.source_type).map(|i| {
                let hash = unquoted_stringify(
                  chunk,
                  &i.rendered(compilation.options.output.hash_digest_length)
                    .to_string(),
                );
                match hash_len_map.get("[contenthash]") {
                  Some(hash_len) => hash[..*hash_len].to_string(),
                  None => hash,
                }
              });

              let hash = match hash_len_map
                .get("[fullhash]")
                .or(hash_len_map.get("[hash]"))
              {
                Some(hash_len) => format!(
                  "\" + {}().slice(0, {}) + \"",
                  RuntimeGlobals::GET_FULL_HASH,
                  hash_len
                ),
                None => format!("\" + {}() + \"", RuntimeGlobals::GET_FULL_HASH),
              };

              let filename = format!(
                "\"{}\"",
                compilation.get_path(
                  &fake_filename,
                  PathData::default()
                    .chunk(&fake_chunk)
                    .hash_optional(Some(hash.as_str()))
                    .content_hash_optional(content_hash.as_deref())
                ),
              );

              if let Some(chunk_id) = &chunk.id {
                let chunk_url = static_chunk_url.entry(filename).or_insert(Vec::new());
                chunk_url.push(chunk_id);
              }
            }
          }
        }
      }
    }

    let static_url = static_chunk_url
      .iter()
      .map(|(filename, chunk_ids)| {
        let condition = if chunk_ids.len() == 1 {
          format!(
            "chunkId === {}",
            serde_json::to_string(&chunk_ids.first()).expect("invalid json to_string")
          )
        } else {
          let content = chunk_ids
            .iter()
            .map(|chunk_id| {
              format!(
                "{}:1",
                serde_json::to_string(chunk_id).expect("invalid json to_string")
              )
            })
            .join(",");
          format!(r#"{{ {} }}[chunkId]"#, content)
        };
        format!("if ({}) return {};", condition, filename)
      })
      .join("\n");

    RawSource::from(format!(
      "// This function allow to reference chunks
        {} = function (chunkId) {{
          // return url for filenames not based on template
          {}
          // return url for filenames based on template
          return {};
        }};
      ",
      self.global,
      static_url,
      dynamic_chunk_url.unwrap_or_else(|| format!("'' + chunkId + '.{}'", self.content_type))
    ))
    .boxed()
  }

  fn attach(&mut self, chunk: ChunkUkey) {
    self.chunk = Some(chunk);
  }
}

impl_runtime_module!(GetChunkFilenameRuntimeModule);
