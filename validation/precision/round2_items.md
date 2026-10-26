### Item 1
rule: eager-fallback-alloc
file: zellij-org_zellij/zellij-server/src/plugins/zellij_exports.rs:1463
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
 1457                  (PermissionStatus::Denied, permission) => {
 1458                      log::error!(
 1459                          "Plugin '{}' permission '{}' denied - Command '{:?}' denied",
 1460                          env.name(),
 1461                          permission
 1462                              .map(|p| p.to_string())
 1463>                             .unwrap_or("UNKNOWN".to_owned()),
 1464                          CommandType::from_str(&command.to_string()).with_context(err_context)?
 1465                      );
 1466                  },
 1467              };
 1468              Ok(())
 1469          })
```

### Item 2
rule: eager-fallback-alloc
file: meilisearch_meilisearch/crates/milli/src/vector/embedder/ollama.rs:217
message: `.to_string()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  211          self.rest_embedder.cache()
  212      }
  213  }
  214  
  215  fn get_ollama_path() -> String {
  216      // Important: Hostname not enough, has to be entire path to embeddings endpoint
  217>     std::env::var("MEILI_OLLAMA_URL").unwrap_or("http://localhost:11434/api/embeddings".to_string())
  218  }
  219  
```

### Item 3
rule: eager-fallback-alloc
file: rust-lang_rust-analyzer/crates/ide/src/moniker.rs:384
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  378          },
  379          kind: if krate == from_crate { MonikerKind::Export } else { MonikerKind::Import },
  380          package_information: {
  381              let (name, repo, version) = match krate.origin(db) {
  382                  CrateOrigin::Library { repo, name } => (name, repo, krate.version(db)),
  383                  CrateOrigin::Local { repo, name } => (
  384>                     name.unwrap_or(krate.display_name(db)?.canonical_name().to_owned()),
  385                      repo,
  386                      krate.version(db),
  387                  ),
  388                  CrateOrigin::Rustc { name } => (
  389                      name.clone(),
  390                      Some("https://github.com/rust-lang/rust/".to_owned()),
```

### Item 4
rule: eager-fallback-alloc
file: nushell_nushell/crates/nu-command/src/network/http/client.rs:986
message: `.to_string()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  980      let msg = if response.header("content-type") == Some("application/json") {
  981          // We use a json response as a heuristic to mean the body will contain a relevant error message.
  982          // This can be widened if the assumption is wrong.
  983          response
  984              .body_mut()
  985              .read_to_string()
  986>             .unwrap_or("Cannot read body".to_string())
  987      } else {
  988          response
  989              .status()
  990              .canonical_reason()
  991              .unwrap_or("")
  992              .to_string()
```

### Item 5
rule: eager-fallback-alloc
file: rust-lang_cargo/src/compiler/compilation.rs:483
message: `.to_string()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  477  /// that are only relevant in a context that has a unit
  478  fn fill_rustc_tool_env(mut cmd: ProcessBuilder, unit: &Unit) -> ProcessBuilder {
  479      if unit.target.is_executable() {
  480          let name = unit
  481              .target
  482              .binary_filename()
  483>             .unwrap_or(unit.target.name().to_string());
  484  
  485          cmd.env("CARGO_BIN_NAME", name);
  486      }
  487      cmd.env("CARGO_CRATE_NAME", unit.target.crate_name());
  488      cmd
  489  }
```

### Item 6
rule: eager-fallback-alloc
file: nushell_nushell/crates/nu-lsp/src/hover.rs:154
message: `String::from` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  148              Id::Variable(var_id, _) => {
  149                  let var = working_set.get_variable(var_id);
  150                  let value = var
  151                      .const_val
  152                      .clone()
  153                      .and_then(|v| v.coerce_into_string().ok())
  154>                     .unwrap_or(String::from(if var.mutable {
  155                          "mutable"
  156                      } else {
  157                          "immutable"
  158                      }));
  159                  let contents = format!("```\n{}\n``` \n---\n{}", var.ty, value);
  160                  markdown_hover(contents)
```

### Item 7
rule: eager-fallback-alloc
file: helix-editor_helix/helix-view/src/theme.rs:592
message: `format!` in `.ok_or(..)` allocates even when unused; use `.ok_or_else(|| ..)`
```rust
  586      }
  587  
  588      pub fn parse_underline_style(value: &Value) -> Result<UnderlineStyle, String> {
  589          value
  590              .as_str()
  591              .and_then(|s| s.parse().ok())
  592>             .ok_or(format!("Invalid underline style: {}", value))
  593      }
  594  
  595      pub fn parse_style(&self, style: &mut Style, value: Value) -> Result<(), String> {
  596          if let Value::Table(entries) = value {
  597              for (name, mut value) in entries {
  598                  match name.as_str() {
```

### Item 8
rule: eager-fallback-alloc
file: rust-lang_rust-analyzer/crates/ide/src/view_memory_layout.rs:188
message: `.to_owned()` in `.unwrap_or(..)` allocates even when unused; use `.unwrap_or_else(|| ..)`
```rust
  182                  Definition::Adt(_)
  183                  | Definition::TypeAlias(_)
  184                  | Definition::BuiltinType(_)
  185                  | Definition::SelfType(_) => "[ROOT]".to_owned(),
  186  
  187                  // def is an item
  188>                 def => def.name(db).map(|n| n.as_str().to_owned()).unwrap_or("[ROOT]".to_owned()),
  189              };
  190  
  191              let typename = ty.display(db, display_target).to_string();
  192  
  193              let mut nodes = vec![MemoryLayoutNode {
  194                  item_name,
```

### Item 9
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:624
message: `process::exit` in library code skips destructors; return an error
```rust
  618                  } else {
  619                      eprintln!(
  620                          "Session '{}' not found. The following sessions are active:",
  621                          session_name
  622                      );
  623                      list_sessions(false, false, true);
  624>                     std::process::exit(1);
  625                  }
  626              } else if let Ok(session_name) = envs::get_session_name() {
  627                  session_name
  628              } else {
  629                  eprintln!("Please specify the session name to subscribe to. The following sessions are active:");
  630                  list_sessions(false, false, true);
```

### Item 10
rule: exit-in-library
file: zellij-org_zellij/src/lib.rs:29
message: `process::exit` in library code skips destructors; return an error
```rust
   23  /// are not subject to the plugin permission system, exactly as Zellij's own builtin plugins are.
   24  ///
   25  /// Must be called at most once, before anything else in the process reads configuration.
   26  pub fn run(distribution: Distribution) {
   27      if let Err(e) = set_distribution(distribution) {
   28          eprintln!("Failed to start: {}", e);
   29>         std::process::exit(2);
   30      }
   31      main()
   32  }
   33  
   34  /// Run stock Zellij
   35  pub fn run_zellij() {
```

### Item 11
rule: exit-in-library
file: zellij-org_zellij/zellij-client/src/cli_client.rs:883
message: `process::exit` in library code skips destructors; return an error
```rust
  877              },
  878              Some((ServerToClientMsg::Exit { .. }, _)) => break,
  879              Some((ServerToClientMsg::LogError { lines }, _)) => {
  880                  for line in lines {
  881                      eprintln!("{}", line);
  882                  }
  883>                 process::exit(2);
  884              },
  885              None => break,
  886              _ => {},
  887          }
  888      }
  889  
```

### Item 12
rule: exit-in-library
file: nushell_nushell/crates/nu-engine/src/exit.rs:65
message: `process::exit` in library code skips destructors; return an error
```rust
   59  /// Unlike [`cleanup_exit`], which in a REPL session warns about running jobs and keeps Nu
   60  /// running instead, this always kills them and exits. Use it where Nu exits without returning to
   61  /// a prompt, such as when commands (`-c`) or a script finish or fail, even with `-i`: jobs still
   62  /// in the table would outlive Nu.
   63  pub fn kill_jobs_and_exit(engine_state: &EngineState, exit_code: i32) -> ! {
   64      kill_all_jobs(&engine_state.jobs);
   65>     std::process::exit(exit_code)
   66  }
   67  
   68  /// Kill all background jobs and remove them from the job table, without warning.
   69  ///
   70  /// For when Nu exits without a prompt to warn at; see [`kill_jobs_and_exit`]. A poisoned job
   71  /// table is still cleaned up, since its jobs would otherwise outlive Nu.
```

### Item 13
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:910
message: `process::exit` in library code skips destructors; return an error
```rust
  904                          // file and then, from within this session, trying to open a new zellij
  905                          // session with the same layout. This causes an infinite recursion in the
  906                          // `zellij_server::terminal_bytes::listen` task, flooding the server and
  907                          // clients with infinite `Render` requests.
  908                          if *session_name == val {
  909                              eprintln!("You are trying to attach to the current session (\"{}\"). Zellij does not support nesting a session in itself.", session_
  910>                             process::exit(1);
  911                          }
  912                      }
  913                      match config_options.attach_to_session {
  914                          Some(true) => {
  915                              let client = match session_resolution::attach_by_name(
  916                                  Some(session_name.clone()),
```

### Item 14
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:704
message: `process::exit` in library code skips destructors; return an error
```rust
  698              );
  699              std::process::exit(exit_status);
  700          },
  701          Err(e) => {
  702              eprintln!("{}", e);
  703              log::error!("Error sending action: {}", e);
  704>             std::process::exit(2);
  705          },
  706      }
  707  }
  708  
  709  pub(crate) fn start_client(opts: CliArgs) {
  710      let (
```

### Item 15
rule: exit-in-library
file: zellij-org_zellij/zellij-utils/src/sessions.rs:438
message: `process::exit` in library code skips destructors; return an error
```rust
  432  
  433  pub fn delete_session(name: &str, force: bool) {
  434      // The socket and the cache folder are named by the session id; the index row by the name.
  435      let session_id = resolve_session_id(name).unwrap_or_else(|| name.to_owned());
  436      if let Err(e) = validate_session_folder_name(&session_id) {
  437          eprintln!("{}", e);
  438>         process::exit(1);
  439      }
  440      if force {
  441          use crate::consts::ipc_connect;
  442          let path = &*ZELLIJ_SOCK_DIR.join(&session_id);
  443          let _ = ipc_connect(path).ok().map(|stream| {
  444              #[cfg(windows)]
```

### Item 16
rule: exit-in-library
file: zellij-org_zellij/src/commands.rs:662
message: `process::exit` in library code skips destructors; return an error
```rust
  656          .unwrap_or_default()
  657          .iter()
  658          .map(|s| s.0.clone())
  659          .collect();
  660      if !existing_sessions.contains(&session_name) {
  661          eprintln!("Session '{}' not found", session_name);
  662>         process::exit(zellij_utils::prompt::EXIT_ERROR);
  663      }
  664      let plugin_url = match &config {
  665          Some(config)
  666              if config
  667                  .plugins
  668                  .aliases
```

### Item 17
rule: format-push-string
file: meilisearch_meilisearch/crates/milli/src/update/index_documents/extract/extract_vector_points.rs:1288
message: `+= &format!(..)` allocates a temporary; use `write!(s, ..)`
```rust
 1282  
 1283              let mut hint_count = 0;
 1284  
 1285              for (vector_misspelling, count) in
 1286                  self.possible_embedding_mistakes.vector_mistakes().take(2)
 1287              {
 1288>                 msg += &format!("\n- Hint: try replacing `{vector_misspelling}` by `_vectors` in {count} document(s).");
 1289                  hint_count += 1;
 1290              }
 1291  
 1292              for (embedder_misspelling, count) in self
 1293                  .possible_embedding_mistakes
 1294                  .embedder_mistakes(embedder_name, unused_vectors_distribution)
```

### Item 18
rule: format-push-string
file: rust-lang_cargo/src/bin/cargo/cli.rs:214
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  208      let version = cargo::version();
  209      let mut version_string = format!("cargo {}\n", version);
  210      if is_verbose {
  211          version_string.push_str(&format!("release: {}\n", version.version));
  212          if let Some(ref ci) = version.commit_info {
  213              version_string.push_str(&format!("commit-hash: {}\n", ci.commit_hash));
  214>             version_string.push_str(&format!("commit-date: {}\n", ci.commit_date));
  215          }
  216          writeln!(version_string, "host: {}", env!("RUST_HOST_TARGET")).unwrap();
  217          add_libgit2(&mut version_string);
  218          add_curl(&mut version_string);
  219          add_ssl(&mut version_string);
  220          writeln!(version_string, "os: {}", os_info::get()).unwrap();
```

### Item 19
rule: format-push-string
file: zellij-org_zellij/zellij-window/src/app/engine.rs:510
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  504      let count = members.len();
  505      let mut places: Vec<&Eligible> = members.values().collect();
  506      places.sort_by(|a, b| b.status.cmp(&a.status).then(a.place.cmp(&b.place)));
  507      let listed: Vec<String> = places.iter().take(4).map(|e| e.place.clone()).collect();
  508      let mut body = listed.join(", ");
  509      if count > listed.len() {
  510>         body.push_str(&format!(" and {} more", count - listed.len()));
  511      }
  512      Content {
  513          title: format!(
  514              "{}: {} {}",
  515              zellij_utils::attention::sanitize_display_text(session),
  516              count,
```

### Item 20
rule: format-push-string
file: zellij-org_zellij/default-plugins/configuration/src/action_picker.rs:657
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  651              (ArgPlace::Positional, ArgKind::Number) => {
  652                  let number: i64 = value
  653                      .parse()
  654                      .map_err(|_| format!("{} must be a number", arg.label))?;
  655                  text.push_str(&format!(" {}", number));
  656              },
  657>             (ArgPlace::Positional, _) => text.push_str(&format!(" {}", kdl_text_value(value))),
  658              (ArgPlace::Rest, _) => {
  659                  for word in value.split_whitespace() {
  660                      text.push_str(&format!(" {}", kdl_text_value(word)));
  661                  }
  662              },
  663              (ArgPlace::RestNumbers, _) => {
```

### Item 21
rule: format-push-string
file: getzola_zola/src/cmd/serve.rs:310
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  304  
  305      if let Some((msg, error)) = SERVE_ERROR.lock().unwrap().get_mut() {
  306          // Generate an error message similar to the CLI version in messages::unravel_errors.
  307          let mut error_str = String::new();
  308  
  309          if !msg.is_empty() {
  310>             error_str.push_str(&format!("Error: {msg}\n"));
  311          }
  312  
  313          error_str.push_str(&format!("Error: {error}\n"));
  314  
  315          let mut cause = error.source();
  316          while let Some(e) = cause {
```

### Item 22
rule: format-push-string
file: rust-lang_rust-analyzer/crates/rust-analyzer/src/reload.rs:184
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  178  
  179              if self.config.has_linked_projects() {
  180                  message.push_str(
  181                      "`rust-analyzer.linkedProjects` have been specified, which may be incorrect. Specified project paths:\n",
  182                  );
  183                  message
  184>                     .push_str(&format!("    {}", self.config.linked_manifests().format("\n    ")));
  185                  if self.config.has_linked_project_jsons() {
  186                      message.push_str("\nAdditionally, one or more project jsons are specified")
  187                  }
  188              }
  189              message.push_str("\n\n");
  190          }
```

### Item 23
rule: format-push-string
file: zellij-org_zellij/zellij-window/src/dbus.rs:797
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  791  }
  792  
  793  pub fn auth_line(uid: u32) -> String {
  794      let digits = uid.to_string();
  795      let mut hex = String::with_capacity(digits.len() * 2);
  796      for byte in digits.as_bytes() {
  797>         hex.push_str(&format!("{:02x}", byte));
  798      }
  799      format!("AUTH EXTERNAL {}\r\n", hex)
  800  }
  801  
  802  pub fn match_rule(interface: &str, member: Option<&str>) -> String {
  803      match member {
```

### Item 24
rule: format-push-string
file: zellij-org_zellij/default-plugins/configuration/src/bindings_list.rs:572
message: `push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`
```rust
  566                  .color_range(3, label_start..label_end)
  567              },
  568              entries => {
  569                  let skipped = targets.len() - entries.len();
  570                  let mut question = format!("Delete {} {}?", entries.len(), plural);
  571                  if skipped > 0 {
  572>                     question.push_str(&format!(
  573                          " {} marked {} cannot be deleted and stay.",
  574                          skipped, plural
  575                      ));
  576                  }
  577                  Text::from(question)
  578              },
```

### Item 25
rule: panic-in-drop
file: rayon-rs_rayon/src/collections/vec_deque.rs:155
message: `assert_eq!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  149  }
  150  
  151  impl<T> Drop for Drain<'_, T> {
  152      fn drop(&mut self) {
  153          if self.deque.len() != self.orig_len - self.range.len() {
  154              // We must not have produced, so just call a normal drain to remove the items.
  155>             assert_eq!(self.deque.len(), self.orig_len);
  156              self.deque.drain(self.range.clone());
  157          }
  158      }
  159  }
  160  
```

### Item 26
rule: panic-in-drop
file: rust-lang_cargo/src/util/network/http_async.rs:186
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  180  
  181  impl Drop for Client {
  182      fn drop(&mut self) {
  183          // Close the channel
  184          drop(self.channel.take().unwrap());
  185          // Join the thread
  186>         let _ = self.thread_handle.take().unwrap().join();
  187      }
  188  }
  189  
  190  impl std::fmt::Debug for Client {
  191      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
  192          f.debug_struct("http_async::Client").finish()
```

### Item 27
rule: panic-in-drop
file: rust-lang_cargo/src/util/rustc.rs:309
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  303  impl Drop for Cache {
  304      fn drop(&mut self) {
  305          if !self.dirty {
  306              return;
  307          }
  308          if let Some(ref path) = self.cache_location {
  309>             let json = serde_json::to_string(&self.data).unwrap();
  310              match paths::write(path, json.as_bytes()) {
  311                  Ok(()) => info!("updated rustc info cache"),
  312                  Err(e) => warn!("failed to update rustc info cache: {}", e),
  313              }
  314          }
  315      }
```

### Item 28
rule: panic-in-drop
file: rust-lang_rust-analyzer/crates/profile/src/lib.rs:77
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
   71          #[cfg(feature = "cpu_profiler")]
   72          {
   73              google_cpu_profiler::stop();
   74              let profile_data = std::env::current_dir().unwrap().join("out.profile");
   75              eprintln!("Profile data saved to:\n\n    {}\n", profile_data.display());
   76              let mut cmd = std::process::Command::new("pprof");
   77>             cmd.arg("-svg").arg(std::env::current_exe().unwrap()).arg(&profile_data);
   78              let out = cmd.output();
   79  
   80              match out {
   81                  Ok(out) if out.status.success() => {
   82                      let svg = profile_data.with_extension("svg");
   83                      std::fs::write(&svg, out.stdout).unwrap();
```

### Item 29
rule: panic-in-drop
file: rustls_rustls/rustls/src/msgs/codec.rs:317
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  311              ListLength::U24 { .. } | ListLength::NonZeroU24 { .. } => {
  312                  let len = self.buf.len() - self.len_offset - 3;
  313                  debug_assert!(len <= 0xff_ffff);
  314                  let len_bytes = u32::to_be_bytes(len as u32);
  315                  let out: &mut [u8; 3] = (&mut self.buf[self.len_offset..self.len_offset + 3])
  316                      .try_into()
  317>                     .unwrap();
  318                  out.copy_from_slice(&len_bytes[1..]);
  319              }
  320          }
  321      }
  322  }
  323  
```

### Item 30
rule: panic-in-drop
file: tokio-rs_tokio/tokio/src/runtime/local_runtime/runtime.rs:399
message: `unreachable!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  393          if let LocalRuntimeScheduler::CurrentThread(current_thread) = &mut self.scheduler {
  394              // This ensures that tasks spawned on the current-thread
  395              // runtime are dropped inside the runtime's context.
  396              let _guard = context::try_set_current(&self.handle.inner);
  397              current_thread.shutdown(&self.handle.inner);
  398          } else {
  399>             unreachable!("LocalRuntime only supports current-thread")
  400          }
  401      }
  402  }
  403  
  404  impl std::panic::UnwindSafe for LocalRuntime {}
  405  
```

### Item 31
rule: panic-in-drop
file: tokio-rs_tokio/tokio-util/src/io/simplex.rs:185
message: `.unwrap()` in `Drop::drop`; a panic during unwinding aborts the process
```rust
  179  }
  180  
  181  impl Drop for Sender {
  182      /// This also wakes up the [`Receiver`].
  183      fn drop(&mut self) {
  184          let maybe_waker = {
  185>             let mut inner = self.inner.lock().unwrap();
  186              inner.close_sender()
  187          };
  188  
  189          if let Some(waker) = maybe_waker {
  190              waker.wake();
  191          }
```

### Item 32
rule: panic-in-drop
file: crossbeam-rs_crossbeam/crossbeam-channel/src/select.rs:1375
message: `panic!` in `Drop::drop`; a panic during unwinding aborts the process
```rust
 1369          f.pad("SelectedOperation { .. }")
 1370      }
 1371  }
 1372  
 1373  impl Drop for SelectedOperation<'_> {
 1374      fn drop(&mut self) {
 1375>         panic!("dropped `SelectedOperation` without completing the operation");
 1376      }
 1377  }
 1378  
```

### Item 33
rule: satd-comment
file: nushell_nushell/crates/nu-cli/src/repl.rs:1138
message: `FIXME` marker: self-admitted technical debt
```rust
 1132          );
 1133          return;
 1134      }
 1135  
 1136      stack.add_env_var("OLDPWD".into(), Value::string(cwd.clone(), span));
 1137  
 1138>     //FIXME: this only changes the current scope, but instead this environment variable
 1139      //should probably be a block that loads the information from the state in the overlay
 1140      if let Err(err) = stack.set_cwd(&path) {
 1141          report_shell_error(Some(stack), engine_state, &err);
 1142          return;
 1143      };
 1144      let cwd = Value::string(cwd, span);
```

### Item 34
rule: satd-comment
file: crossbeam-rs_crossbeam/crossbeam-utils/src/atomic/consume.rs:1
message: `TODO` marker: self-admitted technical debt
```rust
    1> // TODO: use target_has_atomic_load_store once stabilized.
    2  
    3  #[cfg(target_has_atomic = "ptr")]
    4  use core::sync::atomic::Ordering;
    5  
    6  /// Trait which allows reading from primitive atomic types with "consume" ordering.
    7  pub trait AtomicConsume {
```

### Item 35
rule: satd-comment
file: tokio-rs_tracing/tracing-futures/src/executor/futures_01.rs:202
message: `TODO` marker: self-admitted technical debt
```rust
  196          T: Executor,
  197      {
  198          fn spawn(
  199              &mut self,
  200              future: Box<dyn Future<Error = (), Item = ()> + 'static + Send>,
  201          ) -> Result<(), SpawnError> {
  202>             // TODO: get rid of double box?
  203              let future = Box::new(self.with_dispatch(future));
  204              self.inner.spawn(future)
  205          }
  206      }
  207  
  208      impl<T, F> TypedExecutor<F> for WithDispatch<T>
```

### Item 36
rule: satd-comment
file: nushell_nushell/crates/nu-command/src/strings/str_/contains.rs:30
message: `TODO` marker: self-admitted technical debt
```rust
   24      }
   25  
   26      fn signature(&self) -> Signature {
   27          Signature::build("str contains")
   28              .input_output_types(vec![
   29                  (Type::String, Type::Bool),
   30>                 // TODO figure out cell-path type behavior
   31                  (Type::table(), Type::table()),
   32                  (Type::record(), Type::record()),
   33                  (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Bool)))
   34              ])
   35              .required("string", SyntaxShape::String, "The substring to find.")
   36              .rest(
```

### Item 37
rule: satd-comment
file: rust-lang_rust-analyzer/crates/parser/src/grammar/items/consts.rs:28
message: `FIXME` marker: self-admitted technical debt
```rust
   22      } else {
   23          // test_err anonymous_static
   24          // static _: i32 = 5;
   25          name(p);
   26      }
   27  
   28>     // FIXME: Recover on statics with generic params/where clause.
   29      if !is_const && p.at(T![<]) {
   30          // test_err generic_static
   31          // static C<i32>: u32 = 0;
   32          p.error("`static` may not have generic parameters");
   33      }
   34      // test generic_const
```

### Item 38
rule: satd-comment
file: rust-lang_rust-analyzer/crates/hir-def/src/item_tree/lower.rs:477
message: `FIXME` marker: self-admitted technical debt
```rust
  471                  // E.g. `use something::{inner}` (prefix is `None`, path is `something`)
  472                  // or `use something::{path::{inner::{innerer}}}` (prefix is `something::path`, path is `inner`)
  473                  Some(path) => {
  474                      match ModPath::from_src(self.db, path, span_for_range) {
  475                          Some(it) => Some(it),
  476                          None => {
  477>                             // FIXME: report errors somewhere
  478                              return None;
  479                          }
  480                      }
  481                  }
  482              };
  483  
```

### Item 39
rule: satd-comment
file: zellij-org_zellij/zellij-server/src/plugins/zellij_exports.rs:6615
message: `TODO` marker: self-admitted technical debt
```rust
 6609  pub fn wasi_read_bytes(plugin_env: &PluginEnv) -> Result<Vec<u8>> {
 6610      wasi_read_string(plugin_env)
 6611          .and_then(|string| serde_json::from_str(&string).map_err(anyError::new))
 6612          .with_context(|| format!("failed to deserialize object from WASI env"))
 6613  }
 6614  
 6615> // TODO: move to permissions?
 6616  pub(crate) fn check_command_permission(
 6617      plugin_env: &PluginEnv,
 6618      command: &PluginCommand,
 6619  ) -> (PermissionStatus, Option<PermissionType>) {
 6620      if plugin_env.plugin.is_builtin() {
 6621          // built-in plugins can do all the things because they're part of the application and
```

### Item 40
rule: satd-comment
file: rust-lang_rust-analyzer/crates/hir-ty/src/infer/expr.rs:2012
message: `FIXME` marker: self-admitted technical debt
```rust
 2006                      };
 2007                      (arg_types.to_vec(), expected_input_tys)
 2008                  }
 2009                  _ => {
 2010                      // Otherwise, there's a mismatch, so clear out what we're expecting, and set
 2011                      // our input types to err_args so we don't blow up the error messages
 2012>                     // FIXME: Emit an error.
 2013                      (vec![self.types.types.error; provided_args.len()], None)
 2014                  }
 2015              }
 2016          } else {
 2017              (formal_input_tys, expected_input_tys)
 2018          };
```

### Item 41
rule: swallowed-error
file: rust-lang_rust-analyzer/crates/ide/src/doc_links.rs:97
message: `.ok();` silences the Result without checking it
```rust
   91      let mut out = String::new();
   92      cmark_with_options(
   93          doc,
   94          &mut out,
   95          CMarkOptions { code_block_token_count: 3, ..Default::default() },
   96      )
   97>     .ok();
   98      out
   99  }
  100  
  101  /// Remove all links in markdown documentation.
  102  pub(crate) fn remove_links(markdown: &str) -> String {
  103      let mut drop_link = false;
```

### Item 42
rule: swallowed-error
file: zellij-org_zellij/zellij-utils/src/setup.rs:32
message: `Err` arm discards the error without handling or logging it
```rust
   26      let mut themes = Themes::default();
   27      for file in ZELLIJ_DEFAULT_THEMES.files() {
   28          if let Some(content) = file.contents_utf8() {
   29              let sourced_from_external_file = true;
   30              match Themes::from_string(&content.to_string(), sourced_from_external_file) {
   31                  Ok(theme) => themes = themes.merge(theme),
   32>                 Err(_) => {},
   33              }
   34          }
   35      }
   36      themes
   37  }
   38  
```

### Item 43
rule: swallowed-error
file: nushell_nushell/crates/nu-command/src/network/http/interruptible_tcp.rs:241
message: `.ok();` silences the Result without checking it
```rust
  235  
  236          Ok(amount > 0)
  237      }
  238  
  239      fn is_open(&mut self) -> bool {
  240          // Try a non-blocking read to check if connection is still alive
  241>         self.stream.set_nonblocking(true).ok();
  242          let mut buf = [0u8; 1];
  243          let result = match self.stream.peek(&mut buf) {
  244              Ok(_) => true,
  245              Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
  246              Err(_) => false,
  247          };
```

### Item 44
rule: swallowed-error
file: alacritty_alacritty/alacritty/src/polling/ipc.rs:211
message: `Err` arm discards the error without handling or logging it
```rust
  205              Ok(socket) => return Ok(socket),
  206              // Delete orphan sockets.
  207              Err(error) if error.kind() == ErrorKind::ConnectionRefused => {
  208                  let _ = fs::remove_file(&path);
  209              },
  210              // Ignore other errors like permission issues.
  211>             Err(_) => (),
  212          }
  213      }
  214  
  215      Err(IoError::new(ErrorKind::NotFound, "no socket found"))
  216  }
  217  
```

### Item 45
rule: swallowed-error
file: zellij-org_zellij/zellij-utils/src/sessions.rs:459
message: `.ok();` silences the Result without checking it
```rust
  453                  }
  454              }
  455              #[cfg(not(windows))]
  456              {
  457                  IpcSenderWithContext::<ClientToServerMsg>::new(stream)
  458                      .send_client_msg(ClientToServerMsg::KillSession)
  459>                     .ok();
  460              }
  461          });
  462      }
  463      if let Ok(index) = crate::session_index::SessionIndex::open_default() {
  464          let _ = index.delete(name);
  465      }
```

### Item 46
rule: swallowed-error
file: helix-editor_helix/helix-term/src/application.rs:1278
message: `.ok();` silences the Result without checking it
```rust
 1272  
 1273      fn restore_term(&mut self) -> std::io::Result<()> {
 1274          use helix_view::graphics::CursorKind;
 1275          self.terminal
 1276              .backend_mut()
 1277              .show_cursor(CursorKind::Block)
 1278>             .ok();
 1279          self.terminal.restore()
 1280      }
 1281  
 1282      #[cfg(all(not(feature = "integration"), not(windows)))]
 1283      pub fn event_stream(&self) -> impl Stream<Item = std::io::Result<TerminalEvent>> + Unpin {
 1284          use termina::{escape::csi, Terminal as _};
```

### Item 47
rule: swallowed-error
file: sharkdp_bat/src/bin/bat/app.rs:175
message: `.ok();` silences the Result without checking it
```rust
  169          };
  170  
  171          let cache_dir = PROJECT_DIRS.cache_dir();
  172          let assets = assets_from_cache_or_binary(use_custom_assets, cache_dir)?;
  173          Controller::new(&help_config, &assets)
  174              .run(inputs, None)
  175>             .ok();
  176  
  177          Ok(())
  178      }
  179  
  180      /// Build argument list with env vars and CLI args (without config file)
  181      fn build_args_without_config() -> Vec<std::ffi::OsString> {
```

### Item 48
rule: swallowed-error
file: rust-lang_cargo/src/util/network/http_async.rs:408
message: `.ok();` silences the Result without checking it
```rust
  402  
  403      /// Adds the request to the `Multi`, or send an error back through the channel.
  404      fn enqueue_request(&mut self, message: Message) {
  405          match self.multi.add2(message.easy) {
  406              Ok(mut handle) => {
  407                  self.token = self.token.wrapping_add(1);
  408>                 handle.set_token(self.token).ok();
  409                  self.handles.insert(self.token, (handle, message.sender));
  410              }
  411              Err(e) => {
  412                  let _ = message.sender.send(Err(e.into()));
  413              }
  414          }
```

### Item 49
rule: too-many-params
file: nushell_nushell/crates/nu_plugin_polars/src/dataframe/command/data/fill_nan.rs:143
message: `cmd_df` takes 6 parameters (max 5)
```rust
  137          }
  138          .map_err(LabeledError::from)
  139          .map(|pd| pd.set_metadata(metadata))
  140      }
  141  }
  142  
  143> fn cmd_df(
  144      plugin: &PolarsPlugin,
  145      engine: &EngineInterface,
  146      call: &EvaluatedCall,
  147      frame: NuDataFrame,
  148      fill: Value,
  149      val_span: Span,
```

### Item 50
rule: too-many-params
file: rust-lang_rust-analyzer/crates/rust-analyzer/src/test_runner.rs:104
message: `new` takes 7 parameters (max 5)
```rust
   98      pub package: String,
   99      pub target: String,
  100      pub kind: TargetKind,
  101  }
  102  
  103  impl CargoTestHandle {
  104>     pub(crate) fn new(
  105          path: Option<&str>,
  106          options: CargoOptions,
  107          root: &AbsPath,
  108          ws_target_dir: Option<&Utf8Path>,
  109          test_target: TestTarget,
  110          sender: Sender<CargoTestMessage>,
```

### Item 51
rule: too-many-params
file: zellij-org_zellij/zellij-server/src/panes/grid.rs:905
message: `position_in_span` takes 6 parameters (max 5)
```rust
  899          None
  900      })
  901  }
  902  
  903  /// Check whether a (row, col) position falls within a display span.
  904  /// The span is inclusive at start and exclusive at end.
  905> fn position_in_span(
  906      row: usize,
  907      col: usize,
  908      start_row: usize,
  909      start_col: usize,
  910      end_row: usize,
  911      end_col: usize,
```

### Item 52
rule: too-many-params
file: meilisearch_meilisearch/crates/milli/src/vector/settings.rs:1249
message: `check_settings` takes 20 parameters (max 5)
```rust
 1243          }
 1244      }
 1245  }
 1246  
 1247  impl EmbeddingSettings {
 1248      #[allow(clippy::too_many_arguments)]
 1249>     pub(crate) fn check_settings(
 1250          embedder_name: &str,
 1251          source: EmbedderSource,
 1252          context: NestingContext,
 1253          model: &Setting<String>,
 1254          revision: &Setting<String>,
 1255          pooling: &Setting<OverridePooling>,
```

### Item 53
rule: too-many-params
file: rust-lang_rust-analyzer/crates/hir-ty/src/mir/lower/as_place.rs:302
message: `lower_overloaded_deref` takes 6 parameters (max 5)
```rust
  296              return Ok(None);
  297          };
  298          result = result.project(ProjectionElem::Deref);
  299          Ok(Some((result, current)))
  300      }
  301  
  302>     fn lower_overloaded_deref(
  303          &mut self,
  304          current: BasicBlockId,
  305          place: Place<'db>,
  306          source_ty: Ty<'db>,
  307          target_ty: Ty<'db>,
  308          span: MirSpan,
```

### Item 54
rule: too-many-params
file: helix-editor_helix/helix-term/src/ui/document.rs:63
message: `render_text` takes 9 parameters (max 5)
```rust
   57          theme,
   58          decorations,
   59      )
   60  }
   61  
   62  #[allow(clippy::too_many_arguments)]
   63> pub fn render_text(
   64      renderer: &mut TextRenderer,
   65      text: RopeSlice<'_>,
   66      anchor: usize,
   67      text_fmt: &TextFormat,
   68      text_annotations: &TextAnnotations,
   69      syntax_highlighter: Option<Highlighter<'_>>,
```

### Item 55
rule: too-many-params
file: meilisearch_meilisearch/crates/meilisearch/src/routes/indexes/facet_search.rs:553
message: `search_local` takes 6 parameters (max 5)
```rust
  547  
  548      query.filter = intersect_index_filters(query.filter.take(), shard_filters);
  549  
  550      search_local(index_scheduler, query, before_search, progress, auth_filter, features).await
  551  }
  552  
  553> async fn search_local(
  554      index_scheduler: Data<IndexScheduler>,
  555      query: PreprocessedQuery<(IndexUid, FacetSearchQuery)>,
  556      before_search: time::OffsetDateTime,
  557      progress: Progress,
  558      auth_filter: AuthFilter,
  559      features: RoFeatures,
```

### Item 56
rule: too-many-params
file: zellij-org_zellij/zellij-server/src/output/mod.rs:1871
message: `new` takes 10 parameters (max 5)
```rust
 1865      pub collect_ansi_pane_contents: bool,
 1866      cursor_coordinates: Option<(usize, usize)>,
 1867      client_cursor: HashMap<ClientId, CursorState>,
 1868  }
 1869  
 1870  impl Output {
 1871>     pub fn new(
 1872          sixel_image_store: Rc<RefCell<SixelImageStore>>,
 1873          character_cell_size: Rc<RefCell<Option<SizeInPixels>>>,
 1874          styled_underlines: bool,
 1875          osc8_hyperlinks: bool,
 1876          kitty_image_store: Rc<RefCell<KittyImageStore>>,
 1877          kitty_host_capabilities: Rc<RefCell<HashMap<ClientId, KittyHostCapability>>>,
```

### Item 57
rule: unwrap-in-result-fn
file: rust-lang_rust-analyzer/xtask/src/publish/notes.rs:30
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
   24          self.skip_blank_lines()?;
   25          self.output.push('\n');
   26  
   27          loop {
   28              let line = self.iter.peek().unwrap().as_deref().map_err(|e| anyhow!("{e}"))?;
   29              if get_title(line).is_some() {
   30>                 let line = self.iter.next().unwrap().unwrap();
   31                  let (level, title) = get_title(&line).unwrap();
   32                  self.write_title(level, title);
   33              } else if get_list_item(line).is_some() {
   34                  self.process_list()?;
   35              } else if line.starts_with('[') {
   36                  self.process_source_code_block(0)?;
```

### Item 58
rule: unwrap-in-result-fn
file: helix-editor_helix/helix-lsp/src/client.rs:1745
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
 1739      pub fn code_actions(
 1740          &self,
 1741          text_document: lsp::TextDocumentIdentifier,
 1742          range: lsp::Range,
 1743          context: lsp::CodeActionContext,
 1744      ) -> Option<impl Future<Output = Result<Option<Vec<lsp::CodeActionOrCommand>>>>> {
 1745>         let capabilities = self.capabilities.get().unwrap();
 1746  
 1747          // Return early if the server does not support code actions.
 1748          match capabilities.code_action_provider {
 1749              Some(
 1750                  lsp::CodeActionProviderCapability::Simple(true)
 1751                  | lsp::CodeActionProviderCapability::Options(_),
```

### Item 59
rule: unwrap-in-result-fn
file: meilisearch_meilisearch/crates/milli/src/update/index_documents/extract/extract_word_pair_proximity_docids.rs:225
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  219              }
  220              Right(key_value) => {
  221                  value_writer.insert(DelAdd::Addition, document_id.to_ne_bytes()).unwrap();
  222                  key_value
  223              }
  224              Both(key_value, _) => {
  225>                 value_writer.insert(DelAdd::Deletion, document_id.to_ne_bytes()).unwrap();
  226                  value_writer.insert(DelAdd::Addition, document_id.to_ne_bytes()).unwrap();
  227                  key_value
  228              }
  229          };
  230  
  231          key_buffer.clear();
```

### Item 60
rule: unwrap-in-result-fn
file: rust-lang_cargo/crates/mdman/src/main.rs:131
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
  125      }
  126      if sources.is_empty() {
  127          bail!("at least one source must be specified");
  128      }
  129      let opts = Options {
  130          format: format.unwrap(),
  131>         output_dir: output.unwrap(),
  132          sources,
  133          url,
  134          man_map,
  135      };
  136      Ok(opts)
  137  }
```

### Item 61
rule: unwrap-in-result-fn
file: rust-lang_rust-analyzer/crates/rust-analyzer/src/cli/analysis_stats.rs:59
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
   53          let mut rayon_pool = rayon::ThreadPoolBuilder::new()
   54              .thread_name(|ix| format!("RayonWorker{}", ix))
   55              .stack_size(stdx::thread::DEFAULT_STACK_SIZE);
   56          if !self.parallel {
   57              rayon_pool = rayon_pool.num_threads(1);
   58          }
   59>         rayon_pool.build_global().unwrap();
   60  
   61          let mut rng = {
   62              let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
   63              Rand32::new(seed)
   64          };
   65  
```

### Item 62
rule: unwrap-in-result-fn
file: rust-lang_regex/regex-automata/src/meta/strategy.rs:1266
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
 1260                  return Ok(Some(hm));
 1261              }
 1262  
 1263              if span.start >= span.end {
 1264                  break;
 1265              }
 1266>             span.start = litmatch.start.checked_add(1).unwrap();
 1267              min_start = litmatch.end;
 1268          }
 1269          Ok(None)
 1270      }
 1271  
 1272      #[cfg_attr(feature = "perf-inline", inline(always))]
```

### Item 63
rule: unwrap-in-result-fn
file: alacritty_alacritty/alacritty_terminal/src/tty/unix.rs:98
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
   92      assert_eq!(entry.pw_uid, uid);
   93  
   94      // Build a borrowed Passwd struct.
   95      Ok(Passwd {
   96          name: unsafe { CStr::from_ptr(entry.pw_name).to_str().unwrap() },
   97          dir: unsafe { CStr::from_ptr(entry.pw_dir).to_str().unwrap() },
   98>         shell: unsafe { CStr::from_ptr(entry.pw_shell).to_str().unwrap() },
   99      })
  100  }
  101  
  102  pub struct Pty {
  103      child: Child,
  104      file: File,
```

### Item 64
rule: unwrap-in-result-fn
file: zellij-org_zellij/zellij-tile/src/shim.rs:3232
message: `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
```rust
 3226      // (name, created_at, read_only)
 3227      let plugin_command = PluginCommand::ListWebLoginTokens;
 3228      let protobuf_plugin_command: ProtobufPluginCommand = plugin_command.try_into().unwrap();
 3229      object_to_stdout(&protobuf_plugin_command.encode_to_vec());
 3230      unsafe { host_run_plugin_command() };
 3231      let list_tokens_response =
 3232>         ListTokensResponse::decode(bytes_from_stdin().unwrap().as_slice()).unwrap();
 3233  
 3234      if let Some(error) = list_tokens_response.error {
 3235          Err(error)
 3236      } else {
 3237          let tokens_with_info = list_tokens_response
 3238              .tokens
```
