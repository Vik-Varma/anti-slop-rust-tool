### Item 1
rule: blocking-in-async
file: meilisearch_meilisearch/crates/xtask/src/bench/workload.rs:62
message: blocking `std::fs::create_dir_all` inside async code; use `tokio::fs` or `spawn_blocking`
```rust
   56          asset_folder,
   57          &mut HashMap::new(),
   58          false,
   59      )
   60      .await?;
   61  
   62>     std::fs::create_dir_all(report_folder)
   63          .with_context(|| format!("could not create report directory at {report_folder}"))?;
   64  
   65      let trace_filename = format!("{report_folder}/{workload_name}-{run_number}-trace.json");
   66      let report_filename = format!("{report_folder}/{workload_name}-{run_number}-report.json");
   67  
   68      let report_handle = start_report(logs_client, trace_filename, &workload.target).await?;
```

### Item 2
rule: blocking-in-async
file: meilisearch_meilisearch/crates/xtask/src/test/workload.rs:189
message: blocking `std::fs::File::create` inside async code; use `tokio::fs::File` or `spawn_blocking`
```rust
  183              self.assets.retain(|_, asset| {
  184                  asset.local_location.as_ref().is_none_or(|a| !a.starts_with("meilisearch-"))
  185              });
  186  
  187              let workload = Workload::Test(self);
  188              let mut file =
  189>                 std::fs::File::create(&args.common.workload_file[0]).with_context(|| {
  190                      format!("could not open {}", args.common.workload_file[0].display())
  191                  })?;
  192              serde_json::to_writer_pretty(&file, &workload).with_context(|| {
  193                  format!("could not write to {}", args.common.workload_file[0].display())
  194              })?;
  195              file.write_all(b"\n").with_context(|| {
```

### Item 3
rule: blocking-in-async
file: zellij-org_zellij/zellij-client/src/web_client/mod.rs:362
message: blocking `std::net::TcpListener::bind` inside async code; use the `tokio::net` equivalent
```rust
  356                              "Must specify both web_server_cert and web_server_key".to_owned()
  357                          )
  358                      },
  359                  };
  360  
  361                  let listener = runtime.block_on(async move {
  362>                     std::net::TcpListener::bind(format!("{}:{}", web_server_ip, web_server_port))
  363                  });
  364                  listener
  365                      .map(|listener| (runtime, listener, tls_config))
  366                      .map_err(|e| e.to_string())
  367              },
  368          )
```

### Item 4
rule: blocking-in-async
file: zellij-org_zellij/zellij-client/src/web_client/ipc_listener.rs:14
message: blocking `std::fs::create_dir_all` inside async code; use `tokio::fs` or `spawn_blocking`
```rust
    8  use zellij_utils::web_server_contract::web_server_contract::InstructionForWebServer as ProtoInstructionForWebServer;
    9  use zellij_utils::web_server_contract::web_server_contract::WebServerResponse as ProtoWebServerResponse;
   10  
   11  pub async fn create_webserver_receiver(
   12      id: &str,
   13  ) -> Result<interprocess::local_socket::tokio::Stream, Box<dyn std::error::Error + Send + Sync>> {
   14>     std::fs::create_dir_all(&WEBSERVER_SOCKET_PATH.as_path())?;
   15      let socket_path = WEBSERVER_SOCKET_PATH.join(format!("{}", id));
   16  
   17      if socket_path.exists() {
   18          tokio::fs::remove_file(&socket_path).await?;
   19      }
   20  
```

### Item 5
rule: blocking-in-async
file: meilisearch_meilisearch/crates/xtask/src/common/assets.rs:130
message: blocking `std::fs::create_dir_all` inside async code; use `tokio::fs` or `spawn_blocking`
```rust
  124              },
  125          }
  126  
  127          // downloading remote
  128          match &asset.remote_location {
  129              Some(location) => {
  130>                 std::fs::create_dir_all(asset_folder).with_context(|| format!("could not create asset folder at {asset_folder}"))?;
  131                  download_tasks.spawn({
  132                      let client = client.clone();
  133                      let name = name.to_string();
  134                      let location = location.to_string();
  135                      let store_filename = store_filename.clone();
  136                      let asset = asset.clone();
```

### Item 6
rule: blocking-in-async
file: zellij-org_zellij/zellij-client/src/web_client/mod.rs:137
message: blocking `std::net::TcpListener::bind` inside async code; use the `tokio::net` equivalent
```rust
  131              config_file_path.clone(),
  132              startup_timeout,
  133          )
  134      } else {
  135          let runtime = Runtime::new().unwrap();
  136          let listener = runtime.block_on(async move {
  137>             std::net::TcpListener::bind(format!("{}:{}", web_server_ip, web_server_port))
  138          });
  139          let tls_config = match (web_server_cert, web_server_key) {
  140              (Some(web_server_cert), Some(web_server_key)) => {
  141                  let tls_config = runtime.block_on(async move {
  142                      RustlsConfig::from_pem_file(
  143                          PathBuf::from(web_server_cert),
```

### Item 7
rule: blocking-in-async
file: meilisearch_meilisearch/crates/xtask/src/common/assets.rs:109
message: blocking `std::fs::File::open` inside async code; use `tokio::fs::File` or `spawn_blocking`
```rust
  103              }
  104          }
  105  
  106          // checking asset store
  107          let store_filename = format!("{}/{}", asset_folder, name);
  108  
  109>         match std::fs::File::open(&store_filename) {
  110              Ok(file) => {
  111                  if check_sha256(name, asset, file)? {
  112                      continue;
  113                  } else {
  114                      tracing::warn!(asset = name, file = store_filename, "found resource for asset in asset store, but hash differed, skipping to remote method")
  115                  }
```

### Item 8
rule: blocking-in-async
file: meilisearch_meilisearch/crates/xtask/src/common/assets.rs:87
message: blocking `std::fs::File::open` inside async code; use `tokio::fs::File` or `spawn_blocking`
```rust
   81      asset_folder: &str,
   82  ) -> anyhow::Result<()> {
   83      let mut download_tasks = tokio::task::JoinSet::new();
   84      for (name, asset) in assets {
   85          // trying local
   86          if let Some(local) = &asset.local_location {
   87>             match std::fs::File::open(local) {
   88                  Ok(file) => {
   89                      if check_sha256(name, asset, file)? {
   90                          continue;
   91                      } else {
   92                          tracing::warn!(asset = name, file = local, "found local resource for asset but hash differed, skipping to asset store");
   93                      }
```

### Item 9
rule: blocking-in-async
file: rust-lang_cargo/src/sources/registry/http_remote.rs:166
message: blocking `std::fs::write` inside async code; use `tokio::fs` or `spawn_blocking`
```rust
  160                  raw_data,
  161                  index_version: _,
  162              } => {
  163                  trace!("config loaded");
  164                  let config = Some(serde_json::from_slice(&raw_data)?);
  165                  if paths::create_dir_all(&config_json_path.parent().unwrap()).is_ok() {
  166>                     if let Err(e) = fs::write(&config_json_path, &raw_data) {
  167                          tracing::debug!("failed to write config.json cache: {}", e);
  168                      }
  169                  }
  170                  Ok(config)
  171              }
  172              LoadResponse::NotFound => Ok(None),
```

### Item 10
rule: blocking-in-async
file: meilisearch_meilisearch/crates/xtask/src/bench/mod.rs:122
message: blocking `std::fs::File::open` inside async code; use `tokio::fs::File` or `spawn_blocking`
```rust
  116              {
  117                  let dashboard_client = dashboard_client.clone();
  118                  let mut dashboard_urls = Vec::new();
  119                  async move {
  120              for workload_file in args.common.workload_file.iter() {
  121                  let workload: Workload = serde_json::from_reader(
  122>                     std::fs::File::open(workload_file)
  123                          .with_context(|| format!("error opening {}", workload_file.display()))?,
  124                  )
  125                  .with_context(|| format!("error parsing {} as JSON", workload_file.display()))?;
  126  
  127                  let Workload::Bench(workload) = workload else {
  128                      bail!("workload file {} is not a bench workload", workload_file.display());
```

### Item 11
rule: borrowed-clone
file: nushell_nushell/crates/nu-cli/src/repl.rs:682
message: `&engine_state . get_config () . hooks . env_change.clone()` copies the value only to borrow it; pass `&engine_state . get_config () . hooks . env_change`
```rust
  676      repl.buffer = line_editor.current_buffer_contents().to_string();
  677      drop(repl);
  678  
  679      // Check all the environment variables they ask for
  680      // fire the "env_change" hook
  681      if let Err(error) = hook::eval_env_change_hook(
  682>         &engine_state.get_config().hooks.env_change.clone(),
  683          engine_state,
  684          &mut stack,
  685      ) {
  686          report_shell_error(None, engine_state, &error)
  687      }
  688      perf!("env-change hook", start_time, use_color);
```

### Item 12
rule: borrowed-clone
file: getzola_zola/components/site/src/lib.rs:471
message: `&p . file . parent.clone()` copies the value only to borrow it; pass `&p . file . parent`
```rust
  465  
  466          // This is needed in the first place because of silly borrow checker
  467          let mut pages_insert_anchors = HashMap::new();
  468          for (_, p) in &self.library.pages {
  469              pages_insert_anchors.insert(
  470                  p.file.path.clone(),
  471>                 self.find_parent_section_insert_anchor(&p.file.parent.clone(), &p.lang),
  472              );
  473          }
  474  
  475          // For rendering content, we do not need the real library since it's not going to be filled
  476          let dummy_library = Library::default();
  477          let renderer = Renderer { tera, config, library: &dummy_library, cache: &self.cache };
```

### Item 13
rule: borrowed-clone
file: rust-lang_rust-analyzer/crates/hir/src/source_analyzer.rs:685
message: `&call.clone()` copies the value only to borrow it; pass `&call`
```rust
  679  
  680      pub(crate) fn resolve_expr_as_callable(
  681          &self,
  682          db: &'db dyn HirDatabase,
  683          call: &ast::Expr,
  684      ) -> Option<Callable<'db>> {
  685>         let (orig, adjusted) = self.type_of_expr(db, &call.clone())?;
  686          adjusted.unwrap_or(orig).as_callable(db)
  687      }
  688  
  689      pub(crate) fn resolve_field(
  690          &self,
  691          field: &ast::FieldExpr,
```

### Item 14
rule: borrowed-clone
file: rust-lang_rust-analyzer/crates/ide-db/src/path_transform.rs:143
message: `&node.clone()` copies the value only to borrow it; pass `&node`
```rust
  137      #[must_use]
  138      pub fn apply_all<'b>(
  139          &self,
  140          nodes: impl IntoIterator<Item = &'b SyntaxNode>,
  141      ) -> Vec<SyntaxNode> {
  142          let ctx = self.build_ctx();
  143>         nodes.into_iter().map(|node| ctx.apply(&node.clone())).collect()
  144      }
  145  
  146      fn prettify_target_node(&self, node: SyntaxNode) -> SyntaxNode {
  147          match self.target_scope.file_id() {
  148              HirFileId::FileId(_) => node,
  149              HirFileId::MacroFile(file_id) => {
```

### Item 15
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1542
message: `dbg!` in production code
```rust
 1536                  }
 1537                  let our_span = cached_file.covered_span;
 1538                  // We need to move to a local span because we're only reading
 1539                  // the specific file contents via self.get_span_contents.
 1540                  let local_span = (span.offset() - start, span.len()).into();
 1541                  if debugging {
 1542>                     dbg!(&local_span);
 1543                  }
 1544                  let span_contents = self.get_span_contents(our_span);
 1545                  if debugging {
 1546                      dbg!(String::from_utf8_lossy(span_contents));
 1547                  }
 1548                  let span_contents = span_contents.read_span(
```

### Item 16
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1521
message: `dbg!` in production code
```rust
 1515          context_lines_before: usize,
 1516          context_lines_after: usize,
 1517      ) -> Result<Box<dyn miette::SpanContents<'b> + 'b>, miette::MietteError> {
 1518          let debugging = std::env::var("MIETTE_DEBUG").is_ok();
 1519          if debugging {
 1520              let finding_span = "Finding span in StateWorkingSet";
 1521>             dbg!(finding_span, span);
 1522          }
 1523          for cached_file in self.files() {
 1524              let (filename, start, end) = (
 1525                  &cached_file.name,
 1526                  cached_file.covered_span.start,
 1527                  cached_file.covered_span.end,
```

### Item 17
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1564
message: `dbg!` in production code
```rust
 1558                  }
 1559  
 1560                  let data = span_contents.data();
 1561                  if &**filename == "<cli>" {
 1562                      if debugging {
 1563                          let success_cli = "Successfully read CLI span";
 1564>                         dbg!(success_cli, String::from_utf8_lossy(data));
 1565                      }
 1566                      return Ok(Box::new(miette::MietteSpanContents::new(
 1567                          data,
 1568                          retranslated,
 1569                          span_contents.line(),
 1570                          span_contents.column(),
```

### Item 18
rule: dbg-macro
file: meilisearch_meilisearch/crates/build-info/src/main.rs:5
message: `dbg!` in production code
```rust
    1  use build_info::BuildInfo;
    2  
    3  fn main() {
    4      let info = BuildInfo::from_build();
    5>     dbg!(info);
    6  }
    7  
```

### Item 19
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1546
message: `dbg!` in production code
```rust
 1540                  let local_span = (span.offset() - start, span.len()).into();
 1541                  if debugging {
 1542                      dbg!(&local_span);
 1543                  }
 1544                  let span_contents = self.get_span_contents(our_span);
 1545                  if debugging {
 1546>                     dbg!(String::from_utf8_lossy(span_contents));
 1547                  }
 1548                  let span_contents = span_contents.read_span(
 1549                      &local_span,
 1550                      context_lines_before,
 1551                      context_lines_after,
 1552                  )?;
```

### Item 20
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1535
message: `dbg!` in production code
```rust
 1529              if debugging {
 1530                  dbg!(&filename, start, end);
 1531              }
 1532              if span.offset() >= start && span.offset() + span.len() <= end {
 1533                  if debugging {
 1534                      let found_file = "Found matching file";
 1535>                     dbg!(found_file);
 1536                  }
 1537                  let our_span = cached_file.covered_span;
 1538                  // We need to move to a local span because we're only reading
 1539                  // the specific file contents via self.get_span_contents.
 1540                  let local_span = (span.offset() - start, span.len()).into();
 1541                  if debugging {
```

### Item 21
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1530
message: `dbg!` in production code
```rust
 1524              let (filename, start, end) = (
 1525                  &cached_file.name,
 1526                  cached_file.covered_span.start,
 1527                  cached_file.covered_span.end,
 1528              );
 1529              if debugging {
 1530>                 dbg!(&filename, start, end);
 1531              }
 1532              if span.offset() >= start && span.offset() + span.len() <= end {
 1533                  if debugging {
 1534                      let found_file = "Found matching file";
 1535                      dbg!(found_file);
 1536                  }
```

### Item 22
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1557
message: `dbg!` in production code
```rust
 1551                      context_lines_after,
 1552                  )?;
 1553                  let content_span = span_contents.span();
 1554                  // Back to "global" indexing
 1555                  let retranslated = (content_span.offset() + start, content_span.len()).into();
 1556                  if debugging {
 1557>                     dbg!(&retranslated);
 1558                  }
 1559  
 1560                  let data = span_contents.data();
 1561                  if &**filename == "<cli>" {
 1562                      if debugging {
 1563                          let success_cli = "Successfully read CLI span";
```

### Item 23
rule: dbg-macro
file: nushell_nushell/crates/nu-protocol/src/engine/state_working_set.rs:1576
message: `dbg!` in production code
```rust
 1570                          span_contents.column(),
 1571                          span_contents.line_count(),
 1572                      )));
 1573                  } else {
 1574                      if debugging {
 1575                          let success_file = "Successfully read file span";
 1576>                         dbg!(success_file);
 1577                      }
 1578                      return Ok(Box::new(miette::MietteSpanContents::new_named(
 1579                          (**filename).to_owned(),
 1580                          data,
 1581                          retranslated,
 1582                          span_contents.line(),
```

### Item 24
rule: dbg-macro
file: zellij-org_zellij/default-plugins/strider/src/main.rs:149
message: `dbg!` in production code
```rust
  143                          should_render = true;
  144                      }
  145                  },
  146                  _ => {},
  147              },
  148              _ => {
  149>                 dbg!("Unknown event {:?}", event);
  150              },
  151          };
  152          should_render
  153      }
  154  
  155      fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
```

### Item 25
rule: eager-fallback-alloc
file: zellij-org_zellij/default-plugins/strider/src/search_view.rs:68
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
   62              if let Some(search_result) = self.search_results.get(i) {
   63                  let is_selected = Some(i) == selected_index_in_range;
   64                  let mut search_result_text = search_result.name();
   65                  let size = search_result
   66                      .size()
   67                      .map(|s| pretty_bytes(s as f64))
   68>                     .unwrap_or("".to_owned());
   69                  if search_result.is_folder() {
   70                      search_result_text.push(self.platform.separator());
   71                  }
   72  
   73                  let search_result_text_width =
   74                      zellij_tile::graphemes::display_width(&search_result_text);
```

### Item 26
rule: eager-fallback-alloc
file: zellij-org_zellij/zellij-server/src/tab/mod.rs:3440
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
 3434                      PaneGeom::default(), // this will be filled out later
 3435                      self.senders
 3436                          .to_plugin
 3437                          .as_ref()
 3438                          .with_context(err_context)?
 3439                          .clone(),
 3440>                     initial_pane_title.unwrap_or("".to_owned()),
 3441                      String::new(),
 3442                      self.sixel_image_store.clone(),
 3443                      self.kitty_image_store.clone(),
 3444                      self.terminal_emulator_colors.clone(),
 3445                      self.terminal_emulator_color_codes.clone(),
 3446                      self.link_handler.clone(),
```

### Item 27
rule: eager-fallback-alloc
file: zellij-org_zellij/zellij-server/src/plugins/wasm_bridge.rs:3610
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
 3604          (PermissionStatus::Denied, permission) => {
 3605              log::error!(
 3606                  "PluginId '{}' permission '{}' is not allowed - Event '{:?}' denied",
 3607                  plugin_id,
 3608                  permission
 3609                      .map(|p| p.to_string())
 3610>                     .unwrap_or("UNKNOWN".to_owned()),
 3611                  EventType::from_str(&event.to_string()).with_context(err_context)?
 3612              );
 3613          },
 3614      }
 3615      Ok(())
 3616  }
```

### Item 28
rule: eager-fallback-alloc
file: nushell_nushell/crates/nu_plugin_polars/src/dataframe/command/aggregation/value_counts.rs:97
message: `.to_string()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
   91  fn command(
   92      plugin: &PolarsPlugin,
   93      engine: &EngineInterface,
   94      call: &EvaluatedCall,
   95      input: PipelineData,
   96  ) -> Result<PipelineData, ShellError> {
   97>     let column = call.get_flag("column")?.unwrap_or("count".to_string());
   98      let parallel = call.has_flag("parallel")?;
   99      let sort = call.has_flag("sort")?;
  100      let normalize = call.has_flag("normalize")?;
  101      let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
  102      let series = df.as_series(call.head)?;
  103  
```

### Item 29
rule: eager-fallback-alloc
file: zellij-org_zellij/zellij-server/src/panes/link_handler.rs:88
message: `.to_string()` in `.map_or(..)` allocates even when unused; use `.map_or_else(|| ..)`
```rust
   82                  let link = self.links.get(&index);
   83  
   84                  let output = link.map(|link| {
   85                      let id = link
   86                          .id
   87                          .as_ref()
   88>                         .map_or("".to_string(), |id| format!("id={}", id));
   89                      format!("\u{1b}]8;{};{}{}", id, link.uri, TERMINATOR)
   90                  });
   91  
   92                  if output.is_none() {
   93                      log::warn!(
   94                          "attempted to output osc8 link start, but id: {} was not found!",
```

### Item 30
rule: eager-fallback-alloc
file: rust-lang_rust-analyzer/crates/ide-assists/src/handlers/generate_enum_variant.rs:148
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  142      }
  143  }
  144  
  145  fn name_from_field(make: &SyntaxFactory, field: &ast::RecordExprField) -> ast::Name {
  146      let text = match field.name_ref() {
  147          Some(it) => it.to_string(),
  148>         None => name_from_field_shorthand(field).unwrap_or("unknown".to_owned()),
  149      };
  150      make.name(&text)
  151  }
  152  
  153  fn name_from_field_shorthand(field: &ast::RecordExprField) -> Option<String> {
  154      let path = match field.expr()? {
```

### Item 31
rule: eager-fallback-alloc
file: starship_starship/src/modules/kubernetes.rs:173
message: `.to_string()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  167      }
  168  
  169      let default_config_file = context.get_home()?.join(".kube").join("config");
  170  
  171      let kube_cfg = context
  172          .get_env("KUBECONFIG")
  173>         .unwrap_or(default_config_file.to_str()?.to_string());
  174  
  175      let raw_kubeconfigs = env::split_paths(&kube_cfg).map(|file| utils::read_file(file).ok());
  176      let kubeconfigs = parse_kubeconfigs(raw_kubeconfigs);
  177  
  178      let current_kube_ctx_name = kubeconfigs.iter().find_map(|v| match v {
  179          Document::Json(json) => get_current_kube_context_name(json),
```

### Item 32
rule: eager-fallback-alloc
file: helix-editor_helix/helix-view/src/theme.rs:585
message: `format!` in `.ok_or(..)` allocates even when unused; use `.ok_or_else(|| ..)`
```rust
  579      }
  580  
  581      pub fn parse_modifier(value: &Value) -> Result<Modifier, String> {
  582          value
  583              .as_str()
  584              .and_then(|s| s.parse().ok())
  585>             .ok_or(format!("Invalid modifier: {}", value))
  586      }
  587  
  588      pub fn parse_underline_style(value: &Value) -> Result<UnderlineStyle, String> {
  589          value
  590              .as_str()
  591              .and_then(|s| s.parse().ok())
```

### Item 33
rule: eager-fallback-alloc
file: zellij-org_zellij/zellij-server/src/session_layout_metadata.rs:699
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  693              Some(Run::Plugin(..)) => {
  694                  let (plugin, _plugin_config) = extract_plugin_and_config(&self.command);
  695                  plugin.map(|p| format!("{}", p))
  696              },
  697              _ => None,
  698          };
  699>         stringified.unwrap_or("N/A".to_owned())
  700      }
  701      pub fn get_pane_id(&self) -> PaneId {
  702          self.pane_id
  703      }
  704      pub fn render_many(
  705          clients_metadata: BTreeMap<ClientId, ClientMetadata>,
```

### Item 34
rule: eager-fallback-alloc
file: nushell_nushell/crates/nu-parser/src/parse_calls.rs:576
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  570                          )
  571                      }
  572                  }
  573              } else {
  574                  let suggestion = did_you_mean(sig.get_names(), &long_name)
  575                      .map(|name| format!("Did you mean: `--{name}`?"))
  576>                     .unwrap_or("Use `--help` to see available flags".to_owned());
  577                  working_set.error(ParseError::UnknownFlag(
  578                      sig.name.clone(),
  579                      long_name.clone(),
  580                      arg_span,
  581                      suggestion,
  582                  ));
```

### Item 35
rule: eager-fallback-alloc
file: zellij-org_zellij/zellij-server/src/tab/mod.rs:3141
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
 3135                      PaneGeom::default(), // this will be filled out later
 3136                      self.senders
 3137                          .to_plugin
 3138                          .as_ref()
 3139                          .with_context(err_context)?
 3140                          .clone(),
 3141>                     initial_pane_title.unwrap_or("".to_owned()),
 3142                      String::new(),
 3143                      self.sixel_image_store.clone(),
 3144                      self.kitty_image_store.clone(),
 3145                      self.terminal_emulator_colors.clone(),
 3146                      self.terminal_emulator_color_codes.clone(),
 3147                      self.link_handler.clone(),
```

### Item 36
rule: eager-fallback-alloc
file: rust-lang_rust-analyzer/crates/rust-analyzer/src/lsp/to_proto.rs:1633
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
 1627          (None, Some((index, code_action_params, version))) => {
 1628              res.data = Some(lsp_ext::CodeActionData {
 1629                  id: format!(
 1630                      "{}:{}:{index}:{}",
 1631                      assist.id.0,
 1632                      assist.id.1.name(),
 1633>                     assist.id.2.map(|x| x.to_string()).unwrap_or("".to_owned())
 1634                  ),
 1635                  code_action_params,
 1636                  version,
 1637              });
 1638          }
 1639          (None, None) => {
```

### Item 37
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:802
message: `process::exit` in library code skips destructors; return an error
```rust
  796                  if args.options.is_some()
  797                      || args.create
  798                      || create_background
  799                      || args.force_run_commands
  800                  {
  801                      eprintln!("Cannot attach to remote session with options.");
  802>                     std::process::exit(2);
  803                  }
  804  
  805                  if !args.initial_command.is_empty() {
  806                      eprintln!("Cannot run an initial command on a remote session.");
  807                      std::process::exit(2);
  808                  }
```

### Item 38
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:140
message: `process::exit` in library code skips destructors; return an error
```rust
  134  
  135  pub(crate) fn delete_session(target_session: &Option<String>, force: bool) {
  136      match target_session {
  137          Some(target_session) => {
  138              assert_dead_session(target_session, force);
  139              delete_session_impl(target_session, force);
  140>             process::exit(0);
  141          },
  142          None => {
  143              println!("Please specify the session name to delete.");
  144              process::exit(1);
  145          },
  146      }
```

### Item 39
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:130
message: `process::exit` in library code skips destructors; return an error
```rust
  124              assert_session(target_session);
  125              kill_session_impl(target_session);
  126              process::exit(0);
  127          },
  128          None => {
  129              println!("Please specify the session name to kill.");
  130>             process::exit(1);
  131          },
  132      }
  133  }
  134  
  135  pub(crate) fn delete_session(target_session: &Option<String>, force: bool) {
  136      match target_session {
```

### Item 40
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:604
message: `process::exit` in library code skips destructors; return an error
```rust
  598                  if *requested != session_name {
  599                      eprintln!(
  600                          "Session '{}' not found. The following sessions are active:",
  601                          requested
  602                      );
  603                      eprintln!("{}", session_name);
  604>                     std::process::exit(1);
  605                  }
  606              }
  607              session_name
  608          },
  609          ActiveSession::Many => {
  610              let existing_sessions: Vec<String> = get_sessions()
```

### Item 41
rule: exit-in-library
file: rust-lang_cargo/src/lib.rs:192
message: `process::exit` in library code skips destructors; return an error
```rust
  186  
  187      let CliError { error, exit_code } = err;
  188      if let Some(error) = error {
  189          display_error(&error, shell);
  190      }
  191  
  192>     std::process::exit(exit_code)
  193  }
  194  
  195  /// Displays an error, and all its causes, to stderr.
  196  pub fn display_error(err: &Error, shell: &mut Shell) {
  197      debug!("display_error; err={:?}", err);
  198      _display_error(err, shell, true);
```

### Item 42
rule: exit-in-library
file: zellij-org_zellij/zellij-utils/src/setup.rs:507
message: `process::exit` in library code skips destructors; return an error
```rust
  501              Self::generate_auto_start(shell);
  502              std::process::exit(0);
  503          }
  504  
  505          if let Some(layout) = &self.dump_layout {
  506              dump_specified_layout(&layout)?;
  507>             std::process::exit(0);
  508          }
  509  
  510          if let Some(swap_layout) = &self.dump_swap_layout {
  511              dump_specified_swap_layout(swap_layout)?;
  512              std::process::exit(0);
  513          }
```

### Item 43
rule: exit-in-library
file: zellij-org_zellij/zellij-client/src/lib.rs:989
message: `process::exit` in library code skips destructors; return an error
```rust
  983          stdout.flush().unwrap();
  984          if exit_status == 0 {
  985              log::info!("{}", e);
  986          } else {
  987              log::error!("{}", e);
  988          };
  989>         std::process::exit(exit_status);
  990      };
  991  
  992      runtime.block_on(run_remote_client_terminal_loop(
  993          os_input.clone(),
  994          connections,
  995          Some(remote_session_name.clone()),
```

### Item 44
rule: exit-in-library
file: zellij-org_zellij/zellij-client/src/web_client/mod.rs:152
message: `process::exit` in library code skips destructors; return an error
```rust
  146                      .await
  147                  });
  148                  let tls_config = match tls_config {
  149                      Ok(tls_config) => tls_config,
  150                      Err(e) => {
  151                          eprintln!("{}", e);
  152>                         std::process::exit(2);
  153                      },
  154                  };
  155                  Some(tls_config)
  156              },
  157              (None, None) => None,
  158              _ => {
```

### Item 45
rule: exit-in-library
file: zellij-org_zellij/zellij-client/src/web_client/mod.rs:160
message: `process::exit` in library code skips destructors; return an error
```rust
  154                  };
  155                  Some(tls_config)
  156              },
  157              (None, None) => None,
  158              _ => {
  159                  eprintln!("Must specify both web_server_cert and web_server_key");
  160>                 std::process::exit(2);
  161              },
  162          };
  163  
  164          match listener {
  165              Ok(listener) => {
  166                  println!(
```

### Item 46
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:990
message: `process::exit` in library code skips destructors; return an error
```rust
  984      }
  985  }
  986  
  987  fn generate_unique_session_name_or_exit() -> String {
  988      let Some(unique_session_name) = generate_unique_session_name() else {
  989          eprintln!("Failed to generate a unique session name, giving up");
  990>         process::exit(1);
  991      };
  992      unique_session_name
  993  }
  994  
  995  pub(crate) fn list_aliases(opts: CliArgs) {
  996      let (config, _layout, _config_options, _config_without_layout, _config_options_without_layout) =
```

### Item 47
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:156
message: `process::exit` in library code skips destructors; return an error
```rust
  150      fn_get_os_input: fn() -> Result<OsInputOutput, std::io::Error>,
  151  ) -> OsInputOutput {
  152      match fn_get_os_input() {
  153          Ok(os_input) => os_input,
  154          Err(e) => {
  155              eprintln!("failed to open terminal:\n{}", e);
  156>             process::exit(1);
  157          },
  158      }
  159  }
  160  
  161  pub(crate) fn start_server(path: PathBuf, debug: bool) {
  162      // Set instance-wide debug mode
```

### Item 48
rule: exit-in-library
file: nushell_nushell/crates/nu-command/src/system/exec.rs:124
message: `process::exit` in library code skips destructors; return an error
```rust
  118              })?;
  119              let status = child.wait().map_err(|err| ShellError::ExternalCommand {
  120                  label: "Failed to wait for child process".into(),
  121                  help: err.to_string(),
  122                  span: call.head,
  123              })?;
  124>             std::process::exit(status.code().expect("status.code() succeeds on Windows"))
  125          }
  126      }
  127  
  128      fn examples(&self) -> Vec<Example<'_>> {
  129          vec![
  130              Example {
```

### Item 49
rule: format-push-string
file: zellij-org_zellij/zellij-utils/src/desktop_integration/platform.rs:722
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  716          text.push_str(&format!(
  717              " It would be copied to {}.",
  718              destination.display()
  719          ));
  720          if let Some(dir) = destination.parent() {
  721              if !location::path_contains(dir) {
  722>                 text.push_str(&format!(
  723                      " {} is not on your PATH, so add it to run `zellij` from a terminal.",
  724                      dir.display()
  725                  ));
  726              }
  727          }
  728          owned_status(Item::CopyBinary, Owner::Nobody, text.trim().to_owned())
```

### Item 50
rule: format-push-string
file: zellij-org_zellij/zellij-server/src/ui/components/nested_list.rs:65
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
   59                  if line_index != 0 {
   60                      format!("\n\r")
   61                  } else {
   62                      "".to_owned()
   63                  }
   64              });
   65>         stringified.push_str(&format!(
   66              "{}{}{:padding$}{bulletin}{}{text}{}",
   67              go_to_row_instruction,
   68              text_style,
   69              " ",
   70              text_style.bold(Some(AnsiCode::On)),
   71              RESET_STYLES
```

### Item 51
rule: format-push-string
file: rust-lang_cargo/src/ops/lockfile.rs:252
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  246      out.push_str(&format!("version = {}\n", &dep["version"]));
  247  
  248      if dep.contains_key("source") {
  249          out.push_str(&format!("source = {}\n", &dep["source"]));
  250      }
  251      if dep.contains_key("checksum") {
  252>         out.push_str(&format!("checksum = {}\n", &dep["checksum"]));
  253      }
  254  
  255      if let Some(s) = dep.get("dependencies") {
  256          let slice = s.as_array().unwrap();
  257  
  258          if !slice.is_empty() {
```

### Item 52
rule: format-push-string
file: rust-lang_cargo/src/ops/lockfile.rs:249
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  243  
  244  fn emit_package(dep: &toml::Table, out: &mut String) {
  245      out.push_str(&format!("name = {}\n", &dep["name"]));
  246      out.push_str(&format!("version = {}\n", &dep["version"]));
  247  
  248      if dep.contains_key("source") {
  249>         out.push_str(&format!("source = {}\n", &dep["source"]));
  250      }
  251      if dep.contains_key("checksum") {
  252          out.push_str(&format!("checksum = {}\n", &dep["checksum"]));
  253      }
  254  
  255      if let Some(s) = dep.get("dependencies") {
```

### Item 53
rule: format-push-string
file: rust-lang_rust-analyzer/crates/ide-assists/src/handlers/convert_two_arm_bool_match_to_matches_macro.rs:68
message: `+= &format!(..)` allocates a temporary; use `write!(s, ..)`
```rust
   62          |builder| {
   63              let mut arm_str = String::new();
   64              if let Some(pat) = &first_arm.pat() {
   65                  format_to!(arm_str, "{pat}");
   66              }
   67              if let Some(guard) = &first_arm.guard() {
   68>                 arm_str += &format!(" {guard}");
   69              }
   70  
   71              let replace_with = match (first_arm_body, second_arm_body) {
   72                  (Literal(true), Literal(false)) => {
   73                      format!("matches!({expr}, {arm_str})")
   74                  }
```

### Item 54
rule: format-push-string
file: rust-lang_cargo/src/workspace/package_id_spec.rs:133
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  127              let mut version_cnt = HashMap::default();
  128              for id in ids {
  129                  *version_cnt.entry(id.version()).or_insert(0) += 1;
  130              }
  131              for id in ids {
  132                  if version_cnt[id.version()] == 1 {
  133>                     msg.push_str(&format!("\n  {}@{}", spec.name(), id.version()));
  134                  } else {
  135                      msg.push_str(&format!("\n  {}", id.to_spec()));
  136                  }
  137              }
  138          }
  139      }
```

### Item 55
rule: format-push-string
file: clap-rs_clap/clap_complete/src/aot/shells/powershell.rs:114
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  108              ));
  109          }
  110      }
  111  
  112      let mut subcommands_cases = String::new();
  113      for command_name in &command_names {
  114>         subcommands_cases.push_str(&format!(
  115              r"
  116          '{command_name}' {{{completions}
  117              break
  118          }}"
  119          ));
  120      }
```

### Item 56
rule: format-push-string
file: rust-lang_cargo/src/workspace/package.rs:544
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  538              format!("{} crate", pending)
  539          } else {
  540              format!("{} crates", pending)
  541          };
  542          match why {
  543              WhyTick::Extracting(krate) => {
  544>                 msg.push_str(&format!(", extracting {} ...", krate));
  545              }
  546              _ => {
  547                  let remaining = self
  548                      .set
  549                      .gctx
  550                      .http_async()
```

### Item 57
rule: format-push-string
file: zellij-org_zellij/zellij-utils/src/sessions.rs:34
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
   28  /// macOS) even under a long temp dir, which a full 36-char UUID would overflow.
   29  /// `SessionIndex::register` guards against the (astronomically unlikely) collision.
   30  pub fn generate_session_id() -> String {
   31      let bytes = Uuid::new_v4().into_bytes();
   32      let mut id = String::with_capacity(crate::consts::SESSION_ID_LENGTH);
   33      for byte in &bytes[..crate::consts::SESSION_ID_LENGTH / 2] {
   34>         id.push_str(&format!("{:02x}", byte));
   35      }
   36      id
   37  }
   38  
   39  fn now_secs() -> u64 {
   40      SystemTime::now()
```

### Item 58
rule: format-push-string
file: zellij-org_zellij/default-plugins/configuration/src/action_picker.rs:691
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  685              },
  686          }
  687      }
  688      if !children.is_empty() {
  689          text.push_str(" {");
  690          for child in children {
  691>             text.push_str(&format!(" {};", child));
  692          }
  693          text.push_str(" }");
  694      }
  695      text.parse::<KdlDocument>()
  696          .map_err(|e| format!("Not valid KDL: {}", e))?;
  697      Ok(text)
```

### Item 59
rule: format-push-string
file: zellij-org_zellij/default-plugins/configuration/src/action_picker.rs:655
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  649                  return Ok(value.to_owned());
  650              },
  651              (ArgPlace::Positional, ArgKind::Number) => {
  652                  let number: i64 = value
  653                      .parse()
  654                      .map_err(|_| format!("{} must be a number", arg.label))?;
  655>                 text.push_str(&format!(" {}", number));
  656              },
  657              (ArgPlace::Positional, _) => text.push_str(&format!(" {}", kdl_text_value(value))),
  658              (ArgPlace::Rest, _) => {
  659                  for word in value.split_whitespace() {
  660                      text.push_str(&format!(" {}", kdl_text_value(word)));
  661                  }
```

### Item 60
rule: format-push-string
file: rust-lang_rust-analyzer/crates/rust-analyzer/src/main_loop.rs:1037
message: `+= &format!(..)` allocates a temporary; use `write!(s, ..)`
```rust
 1031  
 1032                  self.vfs_progress_config_version = config_version;
 1033                  self.vfs_done = state == Progress::End;
 1034  
 1035                  let mut message = format!("{n_done}/{n_total}");
 1036                  if let Some(dir) = dir {
 1037>                     message += &format!(
 1038                          ": {}",
 1039                          match dir.strip_prefix(self.config.workspace_root_for(&dir)) {
 1040                              Some(relative_path) => relative_path.as_utf8_path(),
 1041                              None => dir.as_ref(),
 1042                          }
 1043                      );
```

### Item 61
rule: panic-in-drop
file: rust-lang_cargo/src/util/diagnostic_server.rs:316
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  310      fn drop(&mut self) {
  311          self.done.store(true, Ordering::SeqCst);
  312          // Ignore errors here as this is largely best-effort
  313          if TcpStream::connect(&self.addr).is_err() {
  314              return;
  315          }
  316>         drop(self.thread.take().unwrap().join());
  317      }
  318  }
  319  
```

### Item 62
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/process/unix/pidfd_reaper.rs:204
message: `.expect()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  198  impl<W, Q> Drop for PidfdReaper<W, Q>
  199  where
  200      W: Wait + Unpin,
  201      Q: OrphanQueue<W> + Unpin,
  202  {
  203      fn drop(&mut self) {
  204>         let mut orphan = self.inner.take().expect("inner has gone away").inner;
  205          if let Ok(Some(_)) = orphan.try_wait() {
  206              return;
  207          }
  208  
  209          self.orphan_queue.push_orphan(orphan);
  210      }
```

### Item 63
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/runtime/io/driver/uring.rs:161
message: `.expect()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  155              // Uring is not initialized or not supported.
  156              return;
  157          }
  158  
  159          // Make sure we flush the submission queue before dropping the driver.
  160          if !self.ring_mut().submission().is_empty() {
  161>             self.submit().expect("Internal error when dropping driver");
  162          }
  163  
  164          let mut ops = std::mem::take(&mut self.ops);
  165  
  166          // Remove all completed ops since we don't need to wait for them.
  167          ops.retain(|_, lifecycle| !matches!(lifecycle, Lifecycle::Completed(_)));
```

### Item 64
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/task/local.rs:1018
message: `assert!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
 1012              let queue = self.context.shared.queue.lock().take().unwrap();
 1013              for task in queue {
 1014                  drop(task);
 1015              }
 1016  
 1017              // Safety: called from the thread that owns `LocalSet`
 1018>             assert!(unsafe { self.context.shared.local_state.owned_is_empty() });
 1019          });
 1020      }
 1021  }
 1022  
 1023  // === impl Context ===
 1024  
```

### Item 65
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/runtime/context/runtime_mt.rs:17
message: `assert!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
   11      // Reset in case the closure panics
   12      struct Reset(EnterRuntime);
   13  
   14      impl Drop for Reset {
   15          fn drop(&mut self) {
   16              CONTEXT.with(|c| {
   17>                 assert!(
   18                      !c.runtime.get().is_entered(),
   19                      "closure claimed permanent executor"
   20                  );
   21                  c.runtime.set(self.0);
   22              });
   23          }
```

### Item 66
rule: panic-in-drop
file: seanmonstar_reqwest/src/async_impl/h3_client/pool.rs:70
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
   64      }
   65  }
   66  
   67  impl Drop for ConnectingLock {
   68      fn drop(&mut self) {
   69          if let Some(ConnectingLockInner { key, pool }) = self.0.take() {
   70>             let mut pool = pool.lock().unwrap();
   71              pool.connecting.remove(&key);
   72              trace!("HTTP/3 connecting lock for {:?} is dropped", key);
   73          }
   74      }
   75  }
   76  
```

### Item 67
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/runtime/io/driver/uring.rs:173
message: `.expect()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  167          ops.retain(|_, lifecycle| !matches!(lifecycle, Lifecycle::Completed(_)));
  168  
  169          while !ops.is_empty() {
  170              // Wait until at least one completion is available.
  171              self.ring_mut()
  172                  .submit_and_wait(1)
  173>                 .expect("Internal error when dropping driver");
  174  
  175              for cqe in self.ring_mut().completion() {
  176                  let idx = cqe.user_data() as usize;
  177  
  178                  if let Some(Lifecycle::Cancelled(CancelData::Open(_))) = ops.get_mut(idx) {
  179                      if let Ok(fd) = CqeResult::from(cqe).result {
```

### Item 68
rule: panic-in-drop
file: rust-lang_rust-analyzer/crates/span/src/ast_id.rs:895
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  889                              // and sleep for a bit
  890                              std::thread::sleep(std::time::Duration::from_millis(100));
  891                          }
  892                          // why do this over just a `receiver.iter().for_each(drop)`? To reduce contention on the channel lock.
  893                          // otherwise this thread will constantly wake up and sleep again.
  894                      })
  895>                     .unwrap();
  896                  sender
  897              })
  898              .send((arena, ptr_map, id_map))
  899              .unwrap();
  900      }
  901  }
```

### Item 69
rule: panic-in-drop
file: seanmonstar_reqwest/src/blocking/client.rs:1430
message: `.expect()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
 1424  impl Drop for InnerClientHandle {
 1425      fn drop(&mut self) {
 1426          let id = self
 1427              .thread
 1428              .as_ref()
 1429              .map(|h| h.thread().id())
 1430>             .expect("thread not dropped yet");
 1431  
 1432          trace!("closing runtime thread ({id:?})");
 1433          self.tx.take();
 1434          trace!("signaled close for runtime thread ({id:?})");
 1435          self.thread.take().map(|h| h.join());
 1436          trace!("closed runtime thread ({id:?})");
```

### Item 70
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/runtime/context/runtime.rs:86
message: `assert!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
   80      }
   81  }
   82  
   83  impl Drop for EnterRuntimeGuard {
   84      fn drop(&mut self) {
   85          CONTEXT.with(|c| {
   86>             assert!(c.runtime.get().is_entered());
   87              c.runtime.set(EnterRuntime::NotEntered);
   88              // Replace the previous RNG seed
   89              let mut rng = c.rng.get().unwrap_or_else(FastRand::new);
   90              rng.replace_seed(self.old_seed.clone());
   91              c.rng.set(Some(rng));
   92          });
```

### Item 71
rule: panic-in-drop
file: crossbeam-rs_crossbeam/crossbeam-epoch/src/sync/list.rs:229
message: `assert_eq!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  223          unsafe {
  224              let guard = unprotected();
  225              let mut curr = self.head.load(Relaxed, guard);
  226              while let Some(c) = NonNull::new(curr.as_ptr() as *mut Entry) {
  227                  let succ = c.as_ref().next.load(Relaxed, guard);
  228                  // Verify that all elements have been removed from the list.
  229>                 assert_eq!(succ.tag(), 1);
  230  
  231                  C::finalize(curr.as_ptr(), guard);
  232                  curr = succ;
  233              }
  234          }
  235      }
```

### Item 72
rule: panic-in-drop
file: rust-lang_rust-analyzer/crates/span/src/map.rs:178
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  172                              // and sleep for a bit
  173                              std::thread::sleep(std::time::Duration::from_millis(100));
  174                          }
  175                          // why do this over just a `receiver.iter().for_each(drop)`? To reduce contention on the channel lock.
  176                          // otherwise this thread will constantly wake up and sleep again.
  177                      })
  178>                     .unwrap();
  179                  sender
  180              })
  181              .send(spans)
  182              .unwrap();
  183      }
  184  }
```

### Item 73
rule: placeholder-comment
file: BurntSushi_ripgrep/crates/ignore/src/walk.rs:424
message: placeholder comment ("placeholder implementation"): the code here is likely incomplete
```rust
  418              follow_link: link,
  419              depth,
  420              ino: md.ino(),
  421          })
  422      }
  423  
  424>     // Placeholder implementation to allow compiling on non-standard platforms
  425      // (e.g. wasm32).
  426      #[cfg(not(any(windows, unix)))]
  427      fn from_path(
  428          depth: usize,
  429          pb: PathBuf,
  430          link: bool,
```

### Item 74
rule: placeholder-comment
file: BurntSushi_ripgrep/crates/ignore/src/walk.rs:374
message: placeholder comment ("placeholder implementation"): the code here is likely incomplete
```rust
  368              follow_link: false,
  369              depth,
  370              ino: ent.ino(),
  371          })
  372      }
  373  
  374>     // Placeholder implementation to allow compiling on non-standard platforms
  375      // (e.g. wasm32).
  376      #[cfg(not(any(windows, unix)))]
  377      fn from_entry_os(
  378          depth: usize,
  379          ent: &fs::DirEntry,
  380          ty: fs::FileType,
```

### Item 75
rule: placeholder-comment
file: nushell_nushell/crates/nu-command/src/formats/from/msgpack.rs:2
message: placeholder comment ("implementation here"): the code here is likely incomplete
```rust
    1  // Credit to https://github.com/hulthe/nu_plugin_msgpack for the original idea, though the
    2> // implementation here is unique.
    3  
    4  use std::{
    5      error::Error,
    6      io::{self, Cursor, ErrorKind},
    7      string::FromUtf8Error,
    8  };
```

### Item 76
rule: placeholder-comment
file: nushell_nushell/crates/nu-command/src/formats/to/msgpack.rs:2
message: placeholder comment ("implementation here"): the code here is likely incomplete
```rust
    1  // Credit to https://github.com/hulthe/nu_plugin_msgpack for the original idea, though the
    2> // implementation here is unique.
    3  
    4  use std::io;
    5  
    6  use byteorder::{BigEndian, WriteBytesExt};
    7  use nu_engine::command_prelude::*;
    8  use nu_protocol::{
```

### Item 77
rule: redundant-box
file: zellij-org_zellij/zellij-server/src/panes/grid.rs:6462
message: `Box<Vec<..>>` double-boxes a heap collection; use `Vec` directly
```rust
 6456  
 6457  fn decode_width(width: Option<NonZeroU32>) -> Option<usize> {
 6458      width.map(|width| width.get() as usize - 1)
 6459  }
 6460  
 6461  #[derive(Clone, Default)]
 6462> struct Osc133Markers(Option<Box<Vec<Osc133Marker>>>);
 6463  
 6464  impl Osc133Markers {
 6465      fn as_slice(&self) -> &[Osc133Marker] {
 6466          self.0.as_deref().map(Vec::as_slice).unwrap_or(&[])
 6467      }
 6468      fn iter(&self) -> std::slice::Iter<'_, Osc133Marker> {
```

### Item 78
rule: redundant-box
file: nushell_nushell/crates/nu-protocol/src/errors/labeled_error.rs:36
message: `Box<Vec<..>>` double-boxes a heap collection; use `Vec` directly
```rust
   30      pub url: Option<String>,
   31      /// Additional help for the error, usually a hint about what the user might try
   32      #[serde(default)]
   33      pub help: Option<String>,
   34      /// Errors that are related to or caused this error
   35      #[serde(default)]
   36>     pub inner: Box<Vec<ShellError>>,
   37  }
   38  
   39  impl LabeledError {
   40      /// Create a new plain [`LabeledError`] with the given message.
   41      ///
   42      /// This is usually used builder-style with methods like [`.with_label()`](Self::with_label) to
```

### Item 79
rule: redundant-box
file: nushell_nushell/crates/nu-protocol/src/errors/labeled_error.rs:23
message: `Box<Vec<..>>` double-boxes a heap collection; use `Vec` directly
```rust
   17  #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
   18  pub struct LabeledError {
   19      /// The main message for the error.
   20      pub msg: String,
   21      /// Labeled spans attached to the error, demonstrating to the user where the problem is.
   22      #[serde(default)]
   23>     pub labels: Box<Vec<ErrorLabel>>,
   24      /// A unique machine- and search-friendly error code to associate to the error. (e.g.
   25      /// `nu::shell::missing_config_value`)
   26      #[serde(default)]
   27      pub code: Option<String>,
   28      /// A link to documentation about the error, used in conjunction with `code`
   29      #[serde(default)]
```

### Item 80
rule: satd-comment
file: meilisearch_meilisearch/crates/meilisearch/src/routes/chats/chat_completions.rs:1008
message: `TODO` marker: self-admitted technical debt
```rust
 1002      rtxn: &RoTxn,
 1003      fields_ids_map: &FieldsIdsMap,
 1004      max_values_per_facet: usize,
 1005      search_rules: Option<IndexSearchRules>,
 1006      index_uid: &str,
 1007  ) -> index_scheduler::Result<String> {
 1008>     // TODO: use the progress from the chat completion when we have it
 1009      let progress = Progress::quiet();
 1010      let features = index_scheduler.features();
 1011      let from_milli = |err| index_scheduler::Error::from_milli(err, Some(index_uid.to_string()));
 1012      let universe = 'filter: {
 1013          let Some(search_rules) = search_rules else { break 'filter index.documents_ids(rtxn)? };
 1014          let Some(filter) = search_rules.filter else {
```

### Item 81
rule: satd-comment
file: rust-lang_rust-analyzer/crates/hir-ty/src/infer/expr.rs:1079
message: `FIXME` marker: self-admitted technical debt
```rust
 1073                  }
 1074                  if !missing_mandatory_fields.is_empty() {
 1075                      // FIXME: Emit an error: missing fields.
 1076                  }
 1077              }
 1078              RecordSpread::Expr(base_expr) => {
 1079>                 // FIXME: We are currently creating two branches here in order to maintain
 1080                  // consistency. But they should be merged as much as possible.
 1081                  if self.features.type_changing_struct_update {
 1082                      if matches!(adt_id, AdtId::StructId(_)) {
 1083                          // Make some fresh generic parameters for our ADT type.
 1084                          let fresh_args = self.table.fresh_args_for_item(expr.into(), adt_id.into());
 1085                          // We do subtyping on the FRU fields first, so we can
```

### Item 82
rule: satd-comment
file: zellij-org_zellij/zellij-server/src/tab/mod.rs:2131
message: `TODO` marker: self-admitted technical debt
```rust
 2125                      self.pane_frame_style,
 2126                  );
 2127  
 2128                  self.apply_buffered_instructions().non_fatal();
 2129              },
 2130              Err(e) => {
 2131>                 // TODO: this should only happen due to an erroneous layout created by user
 2132                  // configuration that was somehow not caught in our KDL layout parser
 2133                  // we should still be able to properly recover from this with a useful error
 2134                  // message though
 2135                  log::error!("Failed to apply layout: {}", e);
 2136                  self.tiled_panes.reapply_pane_frames();
 2137                  self.is_pending = false;
```

### Item 83
rule: satd-comment
file: nushell_nushell/crates/nu-command/src/system/run_external.rs:491
message: `FIXME` marker: self-admitted technical debt
```rust
  485                  result.push(arg.into());
  486              } else {
  487                  result.push(arg.into());
  488              }
  489          }
  490  
  491>         // FIXME: do we want to special-case this further? We might accidentally expand when they don't
  492          // intend to
  493          if result.is_empty() {
  494              result.push(arg.into());
  495          }
  496  
  497          Ok(result)
```

### Item 84
rule: satd-comment
file: zellij-org_zellij/zellij-server/src/panes/grid.rs:2652
message: `FIXME` marker: self-admitted technical debt
```rust
 2646                  .with_bg_color(scroll_bg);
 2647              self.scroll_region_up_at_bottom(new_row, false);
 2648              self.kitty_reanchor_all_from_pixels();
 2649              return;
 2650          }
 2651          if self.viewport.len() <= self.cursor.y + 1 {
 2652>             // FIXME: this should add an empty line with the pad_character
 2653              // but for some reason this breaks rendering in various situations
 2654              // it needs to be investigated and fixed
 2655              let new_row = Row::with_capacity(self.width).canonical();
 2656              self.viewport.push_back(new_row);
 2657          }
 2658          if self.cursor.y == self.height.saturating_sub(1) {
```

### Item 85
rule: satd-comment
file: zellij-org_zellij/zellij-server/src/tab/mod.rs:6634
message: `TODO` marker: self-admitted technical debt
```rust
 6628              closed_pane
 6629          } else if let Some(suppressed_key_of_pane) = self
 6630              .suppressed_panes
 6631              .iter()
 6632              .find_map(|(key, (_, pane))| if &pane.pid() == &id { Some(*key) } else { None })
 6633          {
 6634>             // TODO: test this (from the path in screen.rs focus_plugin_pane ~line 2519
 6635              self.suppressed_panes
 6636                  .remove(&suppressed_key_of_pane)
 6637                  .map(|s_p| s_p.1)
 6638          } else {
 6639              None
 6640          }
```

### Item 86
rule: satd-comment
file: hyperium_hyper/src/common/task.rs:13
message: `TODO` marker: self-admitted technical debt
```rust
    7  /// Useful for spin counts, so a future doesn't hog too much time.
    8  pub(crate) fn yield_now(cx: &mut Context<'_>) -> Poll<std::convert::Infallible> {
    9      cx.waker().wake_by_ref();
   10      Poll::Pending
   11  }
   12  
   13> // TODO: replace with `std::task::Waker::noop()` once MSRV >= 1.85
   14  #[cfg(feature = "client")]
   15  fn noop_waker() -> Waker {
   16      const NOOP_RAW_WAKER: RawWaker = RawWaker::new(std::ptr::null(), &NOOP_VTABLE);
   17      const NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(
   18          // `clone` returns the same noop waker again
   19          |_: *const ()| NOOP_RAW_WAKER,
```

### Item 87
rule: satd-comment
file: rust-lang_rust-analyzer/crates/ide-assists/src/handlers/extract_function.rs:1030
message: `FIXME` marker: self-admitted technical debt
```rust
 1024                      break_expr = Some(it);
 1025                  }
 1026                  ast::Expr::ContinueExpr(it) if loop_depth == 0 => {
 1027                      continue_expr = Some(it);
 1028                  }
 1029                  ast::Expr::AwaitExpr(_) => is_async = true,
 1030>                 // FIXME: Do unsafe analysis on expression, sem highlighting knows this so we should be able
 1031                  // to just lift that out of there
 1032                  // expr if unsafe_depth ==0 && expr.is_unsafe => is_unsafe = true,
 1033                  _ => {}
 1034              }
 1035              false
 1036          });
```

### Item 88
rule: satd-comment
file: rust-lang_rust-analyzer/crates/mbe/src/expander/transcriber.rs:233
message: `FIXME` marker: self-admitted technical debt
```rust
  227                      kind: tt::LitKind::Integer,
  228                      suffix_len: 0,
  229                  }));
  230              }
  231              Op::Len { depth } => {
  232                  let length = ctx.nesting.get(ctx.nesting.len() - 1 - depth).map_or(0, |_nest| {
  233>                     // FIXME: to be implemented
  234                      0
  235                  });
  236                  builder.push(tt::Leaf::Literal(tt::Literal {
  237                      text_and_suffix: sym::Integer::get(length),
  238                      span: ctx.call_site,
  239                      kind: tt::LitKind::Integer,
```

### Item 89
rule: satd-comment
file: rust-lang_rust-analyzer/crates/hir-ide/src/diagnostics/match_check.rs:186
message: `XXX` marker: self-admitted technical debt
```rust
  180  
  181              hir_def::hir::Pat::Record { ref args, .. } if variant.is_some() => {
  182                  let variant_data = variant.unwrap().fields(self.db);
  183                  let subpatterns = args
  184                      .iter()
  185                      .map(|field| {
  186>                         // XXX(iDawer): field lookup is inefficient
  187                          variant_data.field(&field.name).map(|lfield_id| FieldPat {
  188                              field: lfield_id,
  189                              pattern: self.lower_pattern(field.pat),
  190                          })
  191                      })
  192                      .collect();
```

### Item 90
rule: satd-comment
file: rust-lang_rust-analyzer/crates/ide-db/src/search.rs:816
message: `FIXME` marker: self-admitted technical debt
```rust
  810                      }
  811                  }
  812  
  813                  completed.insert(current_to_process);
  814  
  815                  total_files_searched += current_to_process_search_scope.entries.len();
  816>                 // FIXME: Maybe this needs to be relative to the project size, or at least to the initial search scope?
  817                  if total_files_searched > 20_000 && completed.len() > 100 {
  818                      // This case is extremely unlikely (even searching for `Vec::new()` on rust-analyzer does not enter
  819                      // here - it searches less than 10,000 files, and it does so in five seconds), but if we get here,
  820                      // we at a risk of entering an almost-infinite loop of growing the aliases list. So just stop and
  821                      // let normal search handle this case.
  822                      tracing::info!(aliases_count = %completed.len(), "too much aliases; leaving fast path");
```

### Item 91
rule: satd-comment
file: rust-lang_rust-analyzer/crates/hir-ty/src/infer/callee.rs:420
message: `FIXME` marker: self-admitted technical debt
```rust
  414                  Expectation::has_type(self.db.const_param_ty(const_param))
  415              } else {
  416                  Expectation::None
  417              };
  418  
  419              self.infer_expr(args[arg_idx as usize], &expected, ExprIsRead::Yes);
  420>             // FIXME: evaluate and unify with the const
  421          }
  422          legacy_const_generics_indices.sort_unstable();
  423          legacy_const_generics_indices
  424      }
  425  
  426      fn confirm_builtin_call(
```

### Item 92
rule: swallowed-error
file: zellij-org_zellij/default-plugins/configuration/src/themes_screen.rs:623
message: `Err` arm discards the error without handling or logging it
```rust
  617                      .push(Effect::ReplaceBlocks(themes_kdl(&themes)));
  618                  if let Some(form) = self.colour_form.as_mut() {
  619                      form.previewed = true;
  620                      form.error = None;
  621                  }
  622              },
  623>             Err(_) => {},
  624          }
  625      }
  626      fn fix_contrast(&mut self, focused_only: bool) {
  627          let Some(form) = self.colour_form.as_mut() else {
  628              return;
  629          };
```

### Item 93
rule: swallowed-error
file: rust-lang_regex/fuzz/fuzz_targets/ast_fuzz_regex.rs:24
message: `.ok();` silences the Result without checking it
```rust
   18  }
   19  
   20  fuzz_target!(|data: FuzzData| {
   21      let _ = env_logger::try_init();
   22  
   23      let pattern = format!("{}", data.ast);
   24>     RegexBuilder::new(&pattern).size_limit(1 << 20).build().ok();
   25  });
   26  
```

### Item 94
rule: swallowed-error
file: zellij-org_zellij/zellij-window/src/links.rs:207
message: `.ok();` silences the Result without checking it
```rust
  201  fn reap(mut child: std::process::Child) {
  202      std::thread::Builder::new()
  203          .name("zellij-window-open".to_owned())
  204          .spawn(move || {
  205              let _ = child.wait();
  206          })
  207>         .ok();
  208  }
  209  
  210  #[cfg(test)]
  211  thread_local! {
  212      static OPENED: std::cell::RefCell<Vec<(String, String)>> =
  213          std::cell::RefCell::new(Vec::new());
```

### Item 95
rule: swallowed-error
file: rust-lang_rust-analyzer/crates/ide/src/doc_links.rs:132
message: `.ok();` silences the Result without checking it
```rust
  126      let mut out = String::new();
  127      cmark_with_options(
  128          doc,
  129          &mut out,
  130          CMarkOptions { code_block_token_count: 3, ..Default::default() },
  131      )
  132>     .ok();
  133      out
  134  }
  135  
  136  // Feature: Open Docs
  137  //
  138  // Retrieve a links to documentation for the given symbol.
```

### Item 96
rule: swallowed-error
file: rust-lang_rust-analyzer/crates/hir/src/lib.rs:5478
message: `Err` arm discards the error without handling or logging it
```rust
 5472                                  }
 5473                                  method_resolution::PickKind::TraitPick(..) => {
 5474                                      _ = callback.on_trait_method(id);
 5475                                  }
 5476                              }
 5477                          }
 5478>                         Err(_) => {}
 5479                      };
 5480                  }
 5481                  None => {
 5482                      _ = ctx.probe_all(method_resolution::Mode::MethodCall, self_ty).try_for_each(
 5483                          |candidate| {
 5484                              let method_resolution::CandidateId::FunctionId(id) =
```

### Item 97
rule: swallowed-error
file: tokio-rs_axum/axum-core/src/body.rs:181
message: `Err` arm discards the error without handling or logging it
```rust
  175      #[inline]
  176      fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
  177          loop {
  178              match ready!(Pin::new(&mut self.inner).poll_frame(cx)?) {
  179                  Some(frame) => match frame.into_data() {
  180                      Ok(data) => return Poll::Ready(Some(Ok(data))),
  181>                     Err(_frame) => {}
  182                  },
  183                  None => return Poll::Ready(None),
  184              }
  185          }
  186      }
  187  }
```

### Item 98
rule: swallowed-error
file: sharkdp_bat/src/bin/bat/main.rs:238
message: `.ok();` silences the Result without checking it
```rust
  232                  "{}{default_theme_info}\n\n",
  233                  Style::new().bold().paint(theme.to_string()),
  234              ))?;
  235              config.theme = theme.to_string();
  236              Controller::new(&config, &assets)
  237                  .run(vec![theme_preview_file()], Some(&mut handle))
  238>                 .ok();
  239              handle.write_fmt(format_args!("\n"))?;
  240          } else if config.loop_through {
  241              handle.write_fmt(format_args!("{theme}\n"))?;
  242          } else {
  243              handle.write_fmt(format_args!("{theme}{default_theme_info}\n"))?;
  244          }
```

### Item 99
rule: swallowed-error
file: zellij-org_zellij/zellij-utils/src/desktop_integration/desktop_entry.rs:120
message: `Err` arm discards the error without handling or logging it
```rust
  114              Ok(_) => {
  115                  println!(
  116                      "{} was not written by Zellij, so it was left alone",
  117                      entry.display()
  118                  );
  119              },
  120>             Err(_) => {},
  121          }
  122          for (size, _) in icons() {
  123              remove(&icon_path(data_home, size), &mut removed)?;
  124          }
  125          removed.extend(remove_old_launcher(data_home)?);
  126          Ok(removed)
```

### Item 100
rule: swallowed-error
file: nushell_nushell/crates/nu-command/src/network/http/interruptible_tcp.rs:248
message: `.ok();` silences the Result without checking it
```rust
  242          let mut buf = [0u8; 1];
  243          let result = match self.stream.peek(&mut buf) {
  244              Ok(_) => true,
  245              Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
  246              Err(_) => false,
  247          };
  248>         self.stream.set_nonblocking(false).ok();
  249          result
  250      }
  251  }
  252  
  253  impl fmt::Debug for InterruptibleTcpTransport {
  254      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
```

### Item 101
rule: swallowed-error
file: rustls_rustls/rustls/src/msgs/mod.rs:119
message: `.ok();` silences the Result without checking it
```rust
  113          for record in frg.fragment(record.typ, record.version, record.payload.bytes().into(), 0) {
  114              Message::try_from(&Record {
  115                  typ: record.typ,
  116                  version: record.version,
  117                  payload: Payload::Owned(record.payload.to_vec()),
  118              })
  119>             .ok();
  120          }
  121      }
  122  
  123      pub fn fuzz_message(data: &[u8]) {
  124          let mut rdr = Reader::new(data);
  125          let Ok(record) = Record::<Payload<'_>>::read(&mut rdr) else {
```

### Item 102
rule: swallowed-error
file: helix-editor_helix/helix-term/src/commands.rs:6776
message: `.ok();` silences the Result without checking it
```rust
 6770          let is_session_leader = unsafe { libc::getpid() == libc::getsid(0) };
 6771  
 6772          // If helix is the session leader, there is nothing to suspend to, so skip
 6773          if is_session_leader {
 6774              return;
 6775          }
 6776>         _cx.block_try_flush_writes().ok();
 6777          signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
 6778      }
 6779  }
 6780  
 6781  fn add_newline_above(cx: &mut Context) {
 6782      add_newline_impl(cx, Open::Above);
```

### Item 103
rule: swallowed-error
file: helix-editor_helix/helix-view/src/handlers/dap.rs:573
message: `.ok();` silences the Result without checking it
```rust
  567                  };
  568  
  569                  if let Some(debugger) = self.debug_adapters.get_client_mut(id) {
  570                      debugger
  571                          .reply(request.seq, &request.command, reply)
  572                          .await
  573>                         .ok();
  574                  }
  575              }
  576          }
  577          true
  578      }
  579  }
```

### Item 104
rule: todo-in-production
file: rust-lang_cargo/src/util/cpu.rs:242
message: function body is only `unimplemented!()`: an unfinished stub
```rust
  236              io::ErrorKind::Other,
  237              "unsupported platform to learn CPU state",
  238          ))
  239      }
  240  
  241      pub fn pct_idle(_prev: &State, _next: &State) -> f64 {
  242>         unimplemented!()
  243      }
  244  }
  245  
```

### Item 105
rule: todo-in-production
file: helix-editor_helix/helix-core/src/transaction.rs:307
message: function body is only `unimplemented!()`: an unfinished stub
```rust
  301      /// Given two changes `A` and `B`, `A.compose(B.map(A))` and
  302      /// `B.compose(A.map(B, true))` will produce the same document. This
  303      /// provides a basic form of [operational
  304      /// transformation](https://en.wikipedia.org/wiki/Operational_transformation),
  305      /// and can be used for collaborative editing.
  306      pub fn map(self, _other: Self) -> Self {
  307>         unimplemented!()
  308      }
  309  
  310      /// Returns a new changeset that reverts this one. Useful for `undo` implementation.
  311      /// The document parameter expects the original document before this change was applied.
  312      pub fn invert(&self, original_doc: &Rope) -> Self {
  313          assert!(original_doc.len_chars() == self.len);
```

### Item 106
rule: todo-in-production
file: rust-lang_rust-analyzer/crates/paths/src/lib.rs:304
message: function body is only `unimplemented!()`: an unfinished stub
```rust
  298      #[deprecated(note = "use Display instead")]
  299      pub fn display(&self) -> ! {
  300          unimplemented!()
  301      }
  302      #[deprecated(note = "use std::fs::metadata().is_ok() instead")]
  303      pub fn exists(&self) -> ! {
  304>         unimplemented!()
  305      }
  306  
  307      pub fn components(&self) -> Utf8Components<'_> {
  308          self.0.components()
  309      }
  310      // endregion:delegate-methods
```

### Item 107
rule: todo-in-production
file: helix-editor_helix/helix-loader/src/grammar.rs:70
message: function body is only `unimplemented!()`: an unfinished stub
```rust
   64  
   65  const BUILD_TARGET: &str = env!("BUILD_TARGET");
   66  const REMOTE_NAME: &str = "origin";
   67  
   68  #[cfg(target_arch = "wasm32")]
   69  pub fn get_language(name: &str) -> Result<Option<Grammar>> {
   70>     unimplemented!()
   71  }
   72  
   73  #[cfg(not(target_arch = "wasm32"))]
   74  pub fn get_language(name: &str) -> Result<Option<Grammar>> {
   75      let mut rel_library_path = PathBuf::new().join("grammars").join(name);
   76      rel_library_path.set_extension(DYLIB_EXTENSION);
```

### Item 108
rule: todo-in-production
file: rust-lang_cargo/src/workspace/source_id.rs:414
message: `todo!` in production code
```rust
  408                      .expect("path sources cannot be remote");
  409                  if crate::workspace::parser::is_embedded(&path) && path.is_file() {
  410                      anyhow::bail!("single file packages cannot be used as dependencies")
  411                  }
  412                  Ok(Box::new(PathSource::new(&path, self, gctx)))
  413              }
  414>             SourceKind::Builtin => todo!("builtin source"),
  415              SourceKind::Registry | SourceKind::SparseRegistry => {
  416                  Ok(Box::new(RegistrySource::remote(self, gctx)?))
  417              }
  418              SourceKind::LocalRegistry => {
  419                  let path = self
  420                      .inner
```

### Item 109
rule: todo-in-production
file: rustls_rustls/rustls-bench/src/main.rs:854
message: `todo!` in production code
```rust
  848  impl ResumptionParam {
  849      fn from_subcommand(cmd: &Command) -> Self {
  850          match cmd {
  851              Command::Handshake { .. } => Self::No,
  852              Command::HandshakeResume { .. } => Self::SessionId,
  853              Command::HandshakeTicket { .. } => Self::Tickets,
  854>             _ => todo!("unhandled subcommand {cmd:?}"),
  855          }
  856      }
  857  
  858      fn as_handshake_kind(&self) -> HandshakeKind {
  859          match *self {
  860              Self::No => HandshakeKind::Full,
```

### Item 110
rule: todo-in-production
file: nushell_nushell/crates/nu-explore/src/explore/pager/mod.rs:458
message: `todo!` in production code
```rust
  452              // what we do we just replace the view.
  453              let value = view_stack.curr_view.as_mut().and_then(|p| p.view.exit());
  454              let transition = command.react(engine_state, stack, pager, value)?;
  455              match transition {
  456                  Transition::Ok => Ok(CmdResult::new(false, false, String::new())),
  457                  Transition::Exit => Ok(CmdResult::new(true, false, String::new())),
  458>                 Transition::Cmd { .. } => todo!("not used so far"),
  459                  Transition::None => panic!("Transition::None not expected from command.react()"),
  460              }
  461          }
  462          Command::View { mut cmd, stackable } => {
  463              // what we do we just replace the view.
  464              let value = view_stack.curr_view.as_mut().and_then(|p| p.view.exit());
```

### Item 111
rule: todo-in-production
file: nushell_nushell/crates/nu-cmd-lang/src/core_commands/error_make.rs:158
message: `todo!` in production code
```rust
  152                  let mut ei = ErrorInfo::from_value((*val).clone().into_value(internal_span))?;
  153                  ei.inner = [ei.inner, inner].concat();
  154  
  155                  ei.labeled(internal_span, show_labels)
  156              }
  157              (_, Value::Error { error, .. }) => *error,
  158>             _ => todo!(),
  159          })
  160      }
  161  }
  162  
  163  #[derive(Debug, Clone, IntoValue, FromValue)]
  164  struct ErrorInfo {
```

### Item 112
rule: todo-in-production
file: rust-lang_rust-analyzer/crates/parser/test_data/parser/ok/0042_ufcs_call_list.rs:7
message: function body is only `unimplemented!()`: an unfinished stub
```rust
    1  // https://github.com/rust-lang/rust-analyzer/issues/596
    2  
    3  struct Foo;
    4  
    5  impl Foo {
    6      fn bar() -> bool {
    7>         unimplemented!()
    8      }
    9  }
   10  
   11  fn baz(_: bool) {}
   12  
   13  fn main() {
```

### Item 113
rule: todo-in-production
file: rust-lang_rust-analyzer/crates/paths/src/lib.rs:300
message: function body is only `unimplemented!()`: an unfinished stub
```rust
  294      }
  295      pub fn as_str(&self) -> &str {
  296          self.0.as_str()
  297      }
  298      #[deprecated(note = "use Display instead")]
  299      pub fn display(&self) -> ! {
  300>         unimplemented!()
  301      }
  302      #[deprecated(note = "use std::fs::metadata().is_ok() instead")]
  303      pub fn exists(&self) -> ! {
  304          unimplemented!()
  305      }
  306  
```

### Item 114
rule: too-many-params
file: nushell_nushell/crates/nu_plugin_polars/src/dataframe/command/data/cast.rs:147
message: `command_lazy` takes 6 parameters (max 5)
```rust
  141  
  142  fn dtype_arg(call: &EvaluatedCall) -> Result<DataType, ShellError> {
  143      let dtype: String = call.req(0)?;
  144      str_to_dtype(&dtype, call.head)
  145  }
  146  
  147> fn command_lazy(
  148      plugin: &PolarsPlugin,
  149      engine: &EngineInterface,
  150      call: &EvaluatedCall,
  151      column_nm: String,
  152      dtype: DataType,
  153      lazy: NuLazyFrame,
```

### Item 115
rule: too-many-params
file: rust-lang_cargo/src/workspace/global_cache_tracker.rs:941
message: `populate_untracked` takes 8 parameters (max 5)
```rust
  935          Ok(())
  936      }
  937  
  938      /// Updates the database to add any files that are currently not tracked
  939      /// (such as when they are downloaded by an older version of cargo).
  940      #[tracing::instrument(skip(conn, now, gctx, base_path, populate_size))]
  941>     fn populate_untracked(
  942          conn: &Connection,
  943          now: Timestamp,
  944          gctx: &GlobalContext,
  945          id_table_name: &str,
  946          id_column_name: &str,
  947          table_name: &str,
```

### Item 116
rule: too-many-params
file: zellij-org_zellij/zellij-server/src/screen.rs:9215
message: `reconfigure` takes 31 parameters (max 5)
```rust
 9209              {
 9210                  tab.return_pane(unplaced_pane, existing_pane_location);
 9211              }
 9212          }
 9213          let _ = self.log_and_report_session_state();
 9214      }
 9215>     pub fn reconfigure(
 9216          &mut self,
 9217          new_keybinds: SharedKeybinds,
 9218          new_default_mode: InputMode,
 9219          theme: Styling,
 9220          simplified_ui: bool,
 9221          default_shell: Option<PathBuf>,
```

### Item 117
rule: too-many-params
file: zellij-org_zellij/zellij-server/src/plugins/mod.rs:1722
message: `pipe_to_all_plugins` takes 6 parameters (max 5)
```rust
 1716          }
 1717      }
 1718      session_layout_metadata.update_plugin_cmds(plugin_ids_to_cmds);
 1719      session_layout_metadata.update_plugin_aliases_in_default_layout(plugin_aliases);
 1720  }
 1721  
 1722> fn pipe_to_all_plugins(
 1723      pipe_source: PipeSource,
 1724      name: &str,
 1725      payload: &Option<String>,
 1726      args: &Option<BTreeMap<String, String>>,
 1727      wasm_bridge: &mut WasmBridge,
 1728      pipe_messages: &mut Vec<(Option<PluginId>, Option<ClientId>, PipeMessage)>,
```

### Item 118
rule: too-many-params
file: zellij-org_zellij/zellij-utils/src/kdl/kdl_layout_parser.rs:1441
message: `assert_no_bare_attributes_in_pane_node_with_template` takes 6 parameters (max 5)
```rust
 1435                      return true;
 1436                  }
 1437              }
 1438          }
 1439          false
 1440      }
 1441>     fn assert_no_bare_attributes_in_pane_node_with_template(
 1442          &self,
 1443          pane_run: &Option<Run>,
 1444          pane_template_run: &Option<Run>,
 1445          args: &Option<Vec<String>>,
 1446          close_on_exit: &Option<bool>,
 1447          start_suspended: &Option<bool>,
```

### Item 119
rule: too-many-params
file: zellij-org_zellij/zellij-server/src/tab/mouse_handler.rs:2430
message: `handle_prompt_jump` takes 6 parameters (max 5)
```rust
 2424                  }
 2425              }
 2426          }
 2427          Ok(MouseEffect::default())
 2428      }
 2429  
 2430>     fn handle_prompt_jump(
 2431          tab: &mut Tab,
 2432          pane_id: PaneId,
 2433          to_previous_prompt: bool,
 2434          app_first: bool,
 2435          event: &MouseEvent,
 2436          client_id: ClientId,
```

### Item 120
rule: too-many-params
file: zellij-org_zellij/default-plugins/share/src/ui_components.rs:449
message: `new` takes 6 parameters (max 5)
```rust
  443      connection_is_unencrypted: bool,
  444      pub clickable_urls: HashMap<CoordinatesInLine, String>,
  445      pub currently_hovering_over_link: bool,
  446  }
  447  
  448  impl CurrentSessionSection {
  449>     pub fn new(
  450          web_server_started: bool,
  451          web_server_ip: Option<IpAddr>,
  452          web_server_port: Option<u16>,
  453          session_name: Option<String>,
  454          web_sharing: WebSharing,
  455          connection_is_unencrypted: bool,
```

### Item 121
rule: too-many-params
file: helix-editor_helix/helix-view/src/view.rs:534
message: `text_pos_at_screen_coords` takes 6 parameters (max 5)
```rust
  528              ));
  529          }
  530  
  531          text_annotations
  532      }
  533  
  534>     pub fn text_pos_at_screen_coords(
  535          &self,
  536          doc: &Document,
  537          row: u16,
  538          column: u16,
  539          fmt: TextFormat,
  540          annotations: &TextAnnotations,
```

### Item 122
rule: too-many-params
file: rust-lang_rust-analyzer/crates/hir-def/src/attrs/docs.rs:649
message: `extend_with_attrs` takes 10 parameters (max 5)
```rust
  643      expander.ast_id_map = old_ast_id_map;
  644      expander.macro_depth -= 1;
  645  
  646      expansion
  647  }
  648  
  649> fn extend_with_attrs<'a, 'db>(
  650      result: &mut Docs,
  651      db: &'db dyn SourceDatabase,
  652      krate: Crate,
  653      node: &SyntaxNode,
  654      file_id: HirFileId,
  655      expect_inner_attrs: bool,
```

### Item 123
rule: too-many-params
file: rust-lang_rust-analyzer/crates/hir-ty/src/lower/path.rs:678
message: `substs_from_args_and_bindings` takes 8 parameters (max 5)
```rust
  672              span,
  673          );
  674          self.ctx.lifetime_elision = old_lifetime_elision;
  675          result
  676      }
  677  
  678>     pub(super) fn substs_from_args_and_bindings(
  679          &mut self,
  680          args_and_bindings: Option<&HirGenericArgs>,
  681          def: GenericDefId,
  682          infer_args: bool,
  683          explicit_self_ty: Option<Ty<'db>>,
  684          generics_source: PathGenericsSource,
```

### Item 124
rule: too-many-params
file: zellij-org_zellij/zellij-server/src/panes/grid.rs:6397
message: `new` takes 6 parameters (max 5)
```rust
 6391      cursor: Cursor,
 6392      sixel_grid: SixelGrid,
 6393      kitty_grid: KittyGrid,
 6394      supports_kitty_keyboard_protocol: bool,
 6395  }
 6396  impl AlternateScreenState {
 6397>     pub fn new(
 6398          lines_above: VecDeque<Row>,
 6399          viewport: VecDeque<Row>,
 6400          cursor: Cursor,
 6401          sixel_grid: SixelGrid,
 6402          kitty_grid: KittyGrid,
 6403          supports_kitty_keyboard_protocol: bool,
```

### Item 125
rule: too-many-params
file: meilisearch_meilisearch/crates/milli/src/update/index_documents/transform.rs:109
message: `new` takes 6 parameters (max 5)
```rust
  103                  .map(|id| (*field, id)),
  104          })
  105          .collect()
  106  }
  107  
  108  impl<'a, 'i> Transform<'a, 'i> {
  109>     pub fn new(
  110          wtxn: &mut heed::RwTxn<'_>,
  111          index: &'i Index,
  112          indexer_settings: &'a IndexerConfig,
  113          index_documents_method: IndexDocumentsMethod,
  114          embedder_ip_policy: &'a http_client::policy::IpPolicy,
  115          _autogenerate_docids: bool,
```

### Item 126
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/util/idle_notified_set.rs:120
message: `unsafe impl Send` does not require `T: Send`; any `T` (e.g. `Rc`) becomes Send
```rust
  114  // future.
  115  unsafe impl<T: Sync> Sync for IdleNotifiedSet<T> {}
  116  
  117  // These impls control when it is safe to create a Waker. Since the waker does
  118  // not allow access to the value in any way (including its destructor), it is
  119  // not necessary for `T` to be Send or Sync.
  120> unsafe impl<T> Send for ListEntry<T> {}
  121  unsafe impl<T> Sync for ListEntry<T> {}
  122  
  123  impl<T> IdleNotifiedSet<T> {
  124      /// Create a new `IdleNotifiedSet`.
  125      pub(crate) fn new() -> Self {
  126          let lists = Mutex::new(ListsInner {
```

### Item 127
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/util/idle_notified_set.rs:121
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
  115  unsafe impl<T: Sync> Sync for IdleNotifiedSet<T> {}
  116  
  117  // These impls control when it is safe to create a Waker. Since the waker does
  118  // not allow access to the value in any way (including its destructor), it is
  119  // not necessary for `T` to be Send or Sync.
  120  unsafe impl<T> Send for ListEntry<T> {}
  121> unsafe impl<T> Sync for ListEntry<T> {}
  122  
  123  impl<T> IdleNotifiedSet<T> {
  124      /// Create a new `IdleNotifiedSet`.
  125      pub(crate) fn new() -> Self {
  126          let lists = Mutex::new(ListsInner {
  127              notified: LinkedList::new(),
```

### Item 128
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/runtime/scheduler/multi_thread/queue.rs:60
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
   54  
   55      /// Elements
   56      buffer: Box<[UnsafeCell<MaybeUninit<task::Notified<T>>>; LOCAL_QUEUE_CAPACITY]>,
   57  }
   58  
   59  unsafe impl<T> Send for Inner<T> {}
   60> unsafe impl<T> Sync for Inner<T> {}
   61  
   62  #[cfg(not(loom))]
   63  const LOCAL_QUEUE_CAPACITY: usize = 256;
   64  
   65  // Shrink the size of the local queue when using loom. This shouldn't impact
   66  // logic, but allows loom to test more edge cases in a reasonable a mount of
```

### Item 129
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio-util/src/sync/reusable_box.rs:93
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
   87      }
   88  }
   89  
   90  // The only method called on self.boxed is poll, which takes &mut self, so this
   91  // struct being Sync does not permit any invalid access to the Future, even if
   92  // the future is not Sync.
   93> unsafe impl<T> Sync for ReusableBoxFuture<'_, T> {}
   94  
   95  impl<T> fmt::Debug for ReusableBoxFuture<'_, T> {
   96      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
   97          f.debug_struct("ReusableBoxFuture").finish()
   98      }
   99  }
```

### Item 130
rule: unbounded-send-sync
file: crossbeam-rs_crossbeam/crossbeam-utils/src/thread.rs:484
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
  478              _marker: PhantomData,
  479          })
  480      }
  481  }
  482  
  483  unsafe impl<T> Send for ScopedJoinHandle<'_, T> {}
  484> unsafe impl<T> Sync for ScopedJoinHandle<'_, T> {}
  485  
  486  /// A handle that can be used to join its scoped thread.
  487  ///
  488  /// This struct is created by the [`Scope::spawn`] method and the
  489  /// [`ScopedThreadBuilder::spawn`] method.
  490  pub struct ScopedJoinHandle<'scope, T> {
```

### Item 131
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/runtime/scheduler/multi_thread/queue.rs:59
message: `unsafe impl Send` does not require `T: Send`; any `T` (e.g. `Rc`) becomes Send
```rust
   53      tail: AtomicUnsignedShort,
   54  
   55      /// Elements
   56      buffer: Box<[UnsafeCell<MaybeUninit<task::Notified<T>>>; LOCAL_QUEUE_CAPACITY]>,
   57  }
   58  
   59> unsafe impl<T> Send for Inner<T> {}
   60  unsafe impl<T> Sync for Inner<T> {}
   61  
   62  #[cfg(not(loom))]
   63  const LOCAL_QUEUE_CAPACITY: usize = 256;
   64  
   65  // Shrink the size of the local queue when using loom. This shouldn't impact
```

### Item 132
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/signal/reusable_box.rs:132
message: `unsafe impl Send` does not require `T: Send`; any `T` (e.g. `Rc`) becomes Send
```rust
  126      fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
  127          Pin::into_inner(self).get_pin().poll(cx)
  128      }
  129  }
  130  
  131  // The future stored inside ReusableBoxFuture<T> must be Send.
  132> unsafe impl<T> Send for ReusableBoxFuture<T> {}
  133  
  134  // The only method called on self.boxed is poll, which takes &mut self, so this
  135  // struct being Sync does not permit any invalid access to the Future, even if
  136  // the future is not Sync.
  137  unsafe impl<T> Sync for ReusableBoxFuture<T> {}
  138  
```

### Item 133
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/signal/reusable_box.rs:137
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
  131  // The future stored inside ReusableBoxFuture<T> must be Send.
  132  unsafe impl<T> Send for ReusableBoxFuture<T> {}
  133  
  134  // The only method called on self.boxed is poll, which takes &mut self, so this
  135  // struct being Sync does not permit any invalid access to the Future, even if
  136  // the future is not Sync.
  137> unsafe impl<T> Sync for ReusableBoxFuture<T> {}
  138  
  139  // Just like a Pin<Box<dyn Future>> is always Unpin, so is this type.
  140  impl<T> Unpin for ReusableBoxFuture<T> {}
  141  
  142  impl<T> Drop for ReusableBoxFuture<T> {
  143      fn drop(&mut self) {
```

### Item 134
rule: unbounded-send-sync
file: crossbeam-rs_crossbeam/crossbeam-deque/src/deque.rs:58
message: `unsafe impl Send` does not require `T: Send`; any `T` (e.g. `Rc`) becomes Send
```rust
   52      ptr: *mut T,
   53  
   54      /// Capacity of the buffer. Always a power of two.
   55      cap: usize,
   56  }
   57  
   58> unsafe impl<T> Send for Buffer<T> {}
   59  
   60  impl<T> Buffer<T> {
   61      /// Allocates a new buffer with the specified capacity.
   62      fn alloc(cap: usize) -> Self {
   63          debug_assert_eq!(cap, cap.next_power_of_two());
   64  
```

### Item 135
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/util/sync_wrapper.rs:18
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
   12  // safety: The SyncWrapper being send allows you to send the inner value across
   13  // thread boundaries.
   14  unsafe impl<T: Send> Send for SyncWrapper<T> {}
   15  
   16  // safety: An immutable reference to a SyncWrapper is useless, so moving such an
   17  // immutable reference across threads is safe.
   18> unsafe impl<T> Sync for SyncWrapper<T> {}
   19  
   20  impl<T> SyncWrapper<T> {
   21      pub(crate) fn new(value: T) -> Self {
   22          Self { value }
   23      }
   24  
```

### Item 136
rule: unbounded-send-sync
file: crossbeam-rs_crossbeam/crossbeam-utils/src/thread.rs:483
message: `unsafe impl Send` does not require `T: Send`; any `T` (e.g. `Rc`) becomes Send
```rust
  477              thread,
  478              _marker: PhantomData,
  479          })
  480      }
  481  }
  482  
  483> unsafe impl<T> Send for ScopedJoinHandle<'_, T> {}
  484  unsafe impl<T> Sync for ScopedJoinHandle<'_, T> {}
  485  
  486  /// A handle that can be used to join its scoped thread.
  487  ///
  488  /// This struct is created by the [`Scope::spawn`] method and the
  489  /// [`ScopedThreadBuilder::spawn`] method.
```

### Item 137
rule: unbounded-send-sync
file: tokio-rs_tokio/tokio/src/util/ptr_expose.rs:18
message: `unsafe impl Sync` does not require `T: Sync`; any `T` (e.g. `Rc`) becomes Sync
```rust
   12      #[cfg(miri)]
   13      map: Mutex<BTreeMap<usize, *const T>>,
   14      _phantom: PhantomData<T>,
   15  }
   16  
   17  // SAFETY: Actually using the pointers is unsafe, so it's sound to transfer them across threads.
   18> unsafe impl<T> Sync for PtrExposeDomain<T> {}
   19  
   20  impl<T> PtrExposeDomain<T> {
   21      pub(crate) const fn new() -> Self {
   22          Self {
   23              #[cfg(miri)]
   24              map: Mutex::const_new(BTreeMap::new()),
```

### Item 138
rule: unused-io-amount
file: helix-editor_helix/helix-term/src/main.rs:128
message: result of `.read()` (bytes processed) is ignored; use `read_exact`
```rust
  122          }
  123          Err(ConfigLoadError::Error(err)) => return Err(Error::new(err)),
  124          Err(ConfigLoadError::BadConfig(err)) => {
  125              eprintln!("Bad config: {}", err);
  126              eprintln!("Press <ENTER> to continue with default config");
  127              use std::io::Read;
  128>             let _ = std::io::stdin().read(&mut []);
  129              Config::default()
  130          }
  131      };
  132  
  133      let workspace_trust =
  134          helix_loader::workspace_trust::WorkspaceTrust::new((&config.editor.workspace_trust).into());
```

### Item 139
rule: unused-io-amount
file: BurntSushi_ripgrep/crates/printer/src/json.rs:558
message: result of `.write()` (bytes processed) is ignored; use `write_all`
```rust
  552      ) -> io::Result<()> {
  553          if self.config.pretty {
  554              json::to_writer_pretty(&mut self.wtr, message)?;
  555          } else {
  556              json::to_writer(&mut self.wtr, message)?;
  557          }
  558>         let _ = self.wtr.write(b"\n")?; // This will always be Ok(1) when successful.
  559          Ok(())
  560      }
  561  }
  562  
  563  impl<W> JSON<W> {
  564      /// Returns true if and only if this printer has written at least one byte
```

### Item 140
rule: unused-io-amount
file: BurntSushi_ripgrep/crates/printer/src/standard.rs:1730
message: result of `.write()` (bytes processed) is ignored; use `write_all`
```rust
 1724              PreludeSeparator::None => {}
 1725              PreludeSeparator::FieldSeparator => {
 1726                  self.std.write(self.field_separator)?;
 1727              }
 1728              PreludeSeparator::PathTerminator => {
 1729                  if let Some(term) = self.config().path_terminator {
 1730>                     self.std.write(&[term])?;
 1731                  }
 1732              }
 1733          }
 1734          self.next_separator = PreludeSeparator::None;
 1735          Ok(())
 1736      }
```

### Item 141
rule: unused-io-amount
file: rustls_rustls/bogo/src/main.rs:574
message: result of `.read()` (bytes processed) is ignored; use `read_exact`
```rust
  568      // assuming we just flush()'d, we will write no more.
  569      let _ = conn.shutdown(net::Shutdown::Write);
  570  
  571      // wait for EOF
  572      let mut buf = [0u8; 32];
  573      while let Ok(p @ 1..) = conn.peek(&mut buf) {
  574>         let _ = conn.read(&mut buf[..p]).unwrap();
  575      }
  576  
  577      let _ = conn.shutdown(net::Shutdown::Read);
  578  }
  579  
  580  #[derive(Debug, Default)]
```

### Item 142
rule: unused-io-amount
file: helix-editor_helix/helix-term/src/main.rs:142
message: result of `.read()` (bytes processed) is ignored; use `read_exact`
```rust
  136      let lang_loader =
  137          helix_core::config::user_lang_loader(&workspace_trust).unwrap_or_else(|err| {
  138              eprintln!("{}", err);
  139              eprintln!("Press <ENTER> to continue with default language config");
  140              use std::io::Read;
  141              // This waits for an enter press.
  142>             let _ = std::io::stdin().read(&mut []);
  143              helix_core::config::default_lang_loader()
  144          });
  145  
  146      // TODO: use the thread local executor to spawn the application task separately from the work pool
  147      let mut app = Application::new(args, config, lang_loader, workspace_trust)
  148          .context("unable to start Helix")?;
```

### Item 143
rule: unused-io-amount
file: rust-lang_rust-analyzer/crates/rust-analyzer/src/cli/progress_report.rs:70
message: result of `.write()` (bytes processed) is ignored; use `write_all`
```rust
   64          );
   65          self.update_text(&text);
   66      }
   67  
   68      fn update_text(&mut self, text: &str) {
   69          let output = render_text_update(&self.text, text);
   70>         let _ = io::stdout().write(output.as_bytes());
   71          let _ = io::stdout().flush();
   72          text.clone_into(&mut self.text);
   73      }
   74  
   75      fn set_value(&mut self, value: f32) {
   76          self.curr = value.clamp(0.0, 1.0);
```

### Item 144
rule: unwrap-in-result-fn
file: rust-lang_rust-analyzer/xtask/src/publish/notes.rs:463
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  457              let (start, end, a_macro) = match get_next_line_component(&mut chars) {
  458                  Component::None => break,
  459                  Component::Text => continue,
  460                  Component::Macro(s, e, m) => (s, e, m),
  461              };
  462              for _ in pos..start {
  463>                 processed.push(src.next().unwrap());
  464              }
  465              processed.push_str(a_macro.process()?.as_str());
  466              for _ in start..end {
  467                  let _ = src.next().unwrap();
  468              }
  469              pos = end;
```

### Item 145
rule: unwrap-in-result-fn
file: alacritty_alacritty/alacritty/src/input/keyboard.rs:392
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  386          };
  387  
  388          if character.chars().count() == 1 {
  389              let shift = self.modifiers.contains(SequenceModifiers::SHIFT);
  390  
  391              let ch = character.chars().next().unwrap();
  392>             let unshifted_ch = if shift { ch.to_lowercase().next().unwrap() } else { ch };
  393  
  394              let alternate_key_code = u32::from(ch);
  395              let mut unicode_key_code = u32::from(unshifted_ch);
  396  
  397              // Try to get the base for keys which change based on modifier, like `1` for `!`.
  398              //
```

### Item 146
rule: unwrap-in-result-fn
file: meilisearch_meilisearch/crates/dump/src/reader/v2/mod.rs:85
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
   79              .map(|line| -> Result<_> { Ok(serde_json::from_str(&line?)?) })
   80              .collect::<Result<Vec<_>>>()?;
   81  
   82          Ok(V2Reader {
   83              metadata,
   84              tasks: BufReader::new(
   85>                 File::open(dump.path().join("updates").join("data.jsonl")).unwrap(),
   86              ),
   87              index_uuid,
   88              dump,
   89          })
   90      }
   91  
```

### Item 147
rule: unwrap-in-result-fn
file: BurntSushi_ripgrep/crates/cli/src/process.rs:113
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  107      ) -> Result<CommandReader, CommandError> {
  108          let mut child = command
  109              .stdout(process::Stdio::piped())
  110              .stderr(process::Stdio::piped())
  111              .spawn()?;
  112          let stderr = if self.async_stderr {
  113>             StderrReader::r#async(child.stderr.take().unwrap())
  114          } else {
  115              StderrReader::sync(child.stderr.take().unwrap())
  116          };
  117          Ok(CommandReader { child, stderr, eof: false })
  118      }
  119  
```

### Item 148
rule: unwrap-in-result-fn
file: rust-lang_rust-analyzer/xtask/src/publish.rs:47
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
   41          .to_string_lossy();
   42  
   43      let mut chars = file_name.chars();
   44      if file_name.len() >= 10
   45          && chars.next().unwrap().is_ascii_digit()
   46          && chars.next().unwrap().is_ascii_digit()
   47>         && chars.next().unwrap().is_ascii_digit()
   48          && chars.next().unwrap().is_ascii_digit()
   49          && chars.next().unwrap() == '-'
   50          && chars.next().unwrap().is_ascii_digit()
   51          && chars.next().unwrap().is_ascii_digit()
   52          && chars.next().unwrap() == '-'
   53          && chars.next().unwrap().is_ascii_digit()
```

### Item 149
rule: unwrap-in-result-fn
file: zellij-org_zellij/zellij-tile/src/shim.rs:2226
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
 2220  
 2221  pub fn save_keybinds_as_preset(new_name: &str) -> Result<String, String> {
 2222      use zellij_utils::plugin_api::plugin_command::copy_keybind_preset_response::Result as CopyResult;
 2223      let plugin_command = PluginCommand::SaveKeybindsAsPreset {
 2224          new_name: new_name.to_owned(),
 2225      };
 2226>     let protobuf_plugin_command: ProtobufPluginCommand = plugin_command.try_into().unwrap();
 2227      object_to_stdout(&protobuf_plugin_command.encode_to_vec());
 2228      unsafe { host_run_plugin_command() };
 2229      let response = bytes_from_stdin()
 2230          .ok()
 2231          .and_then(|bytes| ProtobufCopyKeybindPresetResponse::decode(bytes.as_slice()).ok());
 2232      match response.and_then(|response| response.result) {
```

### Item 150
rule: unwrap-in-result-fn
file: zellij-org_zellij/zellij-tile/src/shim.rs:604
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  598      object_to_stdout(&protobuf_plugin_command.encode_to_vec());
  599      unsafe { host_run_plugin_command() };
  600  
  601      let response =
  602          ProtobufOpenTerminalInPlaceResponse::decode(bytes_from_stdin().unwrap().as_slice())
  603              .unwrap();
  604>     OpenTerminalInPlaceResponse::try_from(response).unwrap()
  605  }
  606  
  607  /// Open a new terminal pane to the specified location on the host filesystem, temporarily
  608  /// replacing the plugin pane
  609  pub fn open_terminal_in_place_of_plugin<P: AsRef<Path>>(
  610      path: P,
```

### Item 151
rule: unwrap-in-result-fn
file: helix-editor_helix/helix-term/src/commands/typed.rs:4220
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
 4214              };
 4215              name_len + alias_len + arg_len
 4216          }
 4217  
 4218          doc.push_str("\nFlags:");
 4219  
 4220>         let max_flag_len = command.signature.flags.iter().map(flag_len).max().unwrap();
 4221  
 4222          for flag in command.signature.flags {
 4223              let mut buf = [0u8; 4];
 4224              let this_flag_len = flag_len(flag);
 4225              write!(
 4226                  doc,
```

### Item 152
rule: unwrap-in-result-fn
file: zellij-org_zellij/zellij-utils/src/setup.rs:584
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  578          if !distribution.is_stock_zellij() {
  579              writeln!(
  580                  &mut message,
  581                  "[DISTRIBUTION]: {} {} (paths and command name: '{}')",
  582                  distribution.display_name, distribution.version, distribution.name
  583              )
  584>             .unwrap();
  585              let bundled_plugins = crate::distribution::own_plugin_names();
  586              if !bundled_plugins.is_empty() {
  587                  writeln!(
  588                      &mut message,
  589                      "[DISTRIBUTION PLUGINS]: {}",
  590                      bundled_plugins.join(", ")
```

### Item 153
rule: unwrap-in-result-fn
file: meilisearch_meilisearch/crates/milli/src/update/index_documents/extract/extract_word_docids.rs:148
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  142              let add_in_exact = settings_diff.new.exact_attributes.contains(&fid)
  143                  || settings_diff.new.disabled_typos_terms.is_exact(w);
  144              buffer.clear();
  145              let mut obkv = KvWriterDelAdd::new(&mut buffer);
  146              obkv.insert(DelAdd::Addition, value)?;
  147              if add_in_exact {
  148>                 exact_word_docids_sorter.insert(w, obkv.into_inner().unwrap())?;
  149              } else {
  150                  word_docids_sorter.insert(w, obkv.into_inner().unwrap())?;
  151              }
  152          }
  153      }
  154  
```

### Item 154
rule: unwrap-in-result-fn
file: meilisearch_meilisearch/crates/milli/src/vector/embedder/openai.rs:222
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  216              cache_cap,
  217              super::rest::ConfigurationSource::OpenAi,
  218              ip_policy,
  219          )?;
  220  
  221          // looking at the code it is very unclear that this can actually fail.
  222>         let tokenizer = tiktoken_rs::cl100k_base().unwrap();
  223  
  224          Ok(Self { options, rest_embedder, tokenizer })
  225      }
  226  
  227      pub fn embed<S: AsRef<str> + serde::Serialize>(
  228          &self,
```

### Item 155
rule: unwrap-in-result-fn
file: meilisearch_meilisearch/crates/meilisearch/src/lib.rs:441
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  435              .spawn(move || loop {
  436                  thread::sleep(snapshot_delay);
  437                  if let Err(e) = index_scheduler.register(KindWithContent::SnapshotCreation) {
  438                      error!("Error while registering snapshot: {}", e);
  439                  }
  440              })
  441>             .unwrap();
  442      }
  443  
  444      Ok((index_scheduler, auth_controller))
  445  }
  446  
  447  /// Returns the input - 1MiB, or at least 20MiB
```
