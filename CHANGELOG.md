# Changelog

## [2.0.0](https://github.com/minodisk/handpic/compare/v1.10.0...v2.0.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* **app:** the app is now Handpic, with the identifier com.minodisk.handpic; a Riffle install's data moves there on first launch.

### Features

* **app:** add a focus peaking toggle on e ([#929](https://github.com/minodisk/handpic/issues/929)) ([f6f3935](https://github.com/minodisk/handpic/commit/f6f39353b3f67dcd6d532f669603bbea2de1d8ff))
* **app:** add the installer migration config and the data-dir migration ([#953](https://github.com/minodisk/handpic/issues/953)) ([117c0c9](https://github.com/minodisk/handpic/commit/117c0c98b256bfe96b11abf95253938883a4f386))
* **app:** cap the scan's concurrent reads at 2 ([#935](https://github.com/minodisk/handpic/issues/935)) ([fb3ac3f](https://github.com/minodisk/handpic/commit/fb3ac3f3af6fa398f1aa7d225168278b22c1ceea))
* **app:** measure and store full-size eye sharpness in pass 2 ([#946](https://github.com/minodisk/handpic/issues/946)) ([9e7bf16](https://github.com/minodisk/handpic/commit/9e7bf168d5500d452a783276e01cf50022694dcb))
* **app:** pause the scan's reads while a preview read is in flight ([#927](https://github.com/minodisk/handpic/issues/927)) ([d49fa25](https://github.com/minodisk/handpic/commit/d49fa25165d518184ba62c7d176b980b9b65a8ff))
* **app:** prefetch and keep the neighbours' decoded previews ([#943](https://github.com/minodisk/handpic/issues/943)) ([0c50c3f](https://github.com/minodisk/handpic/commit/0c50c3ff6f4d18862c6666a928a069fea63c8a28))
* **app:** rename the app to Handpic ([#957](https://github.com/minodisk/handpic/issues/957)) ([4a131a2](https://github.com/minodisk/handpic/commit/4a131a2a19c72adca3ca0d467ede748b69b604ce))
* **app:** show a frowning face for Face focus Low ([#930](https://github.com/minodisk/handpic/issues/930)) ([2b7c09f](https://github.com/minodisk/handpic/commit/2b7c09f843626c2dc59b7b271dd42a1e8cfa5207))
* **app:** take a frame soft at full size out of Face focus Good ([#947](https://github.com/minodisk/handpic/issues/947)) ([2367e67](https://github.com/minodisk/handpic/commit/2367e67ef98de8c528c28b76e7fd0a223eea295f))
* **app:** time every stage of the preview path ([#934](https://github.com/minodisk/handpic/issues/934)) ([d439c58](https://github.com/minodisk/handpic/commit/d439c5862fe93bb423a803a1724b30d6e13dad8b))
* **backend:** add the shared riffle-backend crate with exif, shortcuts and photolab ([#956](https://github.com/minodisk/handpic/issues/956)) ([fb7fcdf](https://github.com/minodisk/handpic/commit/fb7fcdf434cc0378dd111033055efb8efb97fa81))
* **backend:** move the CPU and disk sampler into the backend ([#1002](https://github.com/minodisk/handpic/issues/1002)) ([f611b2c](https://github.com/minodisk/handpic/commit/f611b2cf2f8600d19322d7b97b42cfa4cfbefc96))
* **backend:** move the folder watcher into riffle-backend ([#960](https://github.com/minodisk/handpic/issues/960)) ([aa325d9](https://github.com/minodisk/handpic/commit/aa325d9de00a37920c55aa4bcba5bad584c188e2))
* **backend:** move the index and the sidecar writer into riffle-backend ([#958](https://github.com/minodisk/handpic/issues/958)) ([567734d](https://github.com/minodisk/handpic/commit/567734da613373ec138d54a041395d9fcb0043d3))
* **backend:** move trash, output and diagnostics into riffle-backend ([#959](https://github.com/minodisk/handpic/issues/959)) ([247c61a](https://github.com/minodisk/handpic/commit/247c61a8242e0b093c9466f0804693b73eb3f9a9))
* **cli:** dump the full-size eye sharpness and validate its threshold ([#941](https://github.com/minodisk/handpic/issues/941)) ([5e630d4](https://github.com/minodisk/handpic/commit/5e630d4198c7787c1b2fbc912cc1a19a0ce28113))
* **core:** add the full-size eye sharpness metric ([#937](https://github.com/minodisk/handpic/issues/937)) ([79c42ea](https://github.com/minodisk/handpic/commit/79c42eaf58c871a613b1a1273cee4c1db0014343))
* **native:** add a text input and edit the label names in the Sidecar tab ([#1031](https://github.com/minodisk/handpic/issues/1031)) ([fdcec35](https://github.com/minodisk/handpic/commit/fdcec3573fd59a53e545e9bd83b304d8df7247c3))
* **native:** add focus peaking on e over the fit view and the 1:1 crop ([#982](https://github.com/minodisk/handpic/issues/982)) ([b9645c1](https://github.com/minodisk/handpic/commit/b9645c18a4eae79d9408880a466e29807e7b20bc))
* **native:** add Move to Trash and Delete Permanently with their dialog ([#1032](https://github.com/minodisk/handpic/issues/1032)) ([ce98356](https://github.com/minodisk/handpic/commit/ce983569e9727dbf440e9a7fd7934063ad4b723e))
* **native:** add rejectRest and the MCP companion's burst ([#1049](https://github.com/minodisk/handpic/issues/1049)) ([ff9a536](https://github.com/minodisk/handpic/commit/ff9a536e1a5dd29cedd38e4705d2a959994b1f75))
* **native:** add the 1:1 view on z ([#979](https://github.com/minodisk/handpic/issues/979)) ([fc42787](https://github.com/minodisk/handpic/commit/fc4278753ee6af2f153f2ff1bd91d0a1054392cb))
* **native:** add the folder menu's Import dialog ([#1046](https://github.com/minodisk/handpic/issues/1046)) ([4ee6597](https://github.com/minodisk/handpic/commit/4ee6597689fafaf8be1d008977833aa71dcd07ec))
* **native:** add the folder menu's items and the Rewrite Sidecars / Delete Sidecars dialogs ([#1039](https://github.com/minodisk/handpic/issues/1039)) ([f942b4d](https://github.com/minodisk/handpic/commit/f942b4ddfd09f047c870ef9c6bda53281c0c466f))
* **native:** add the folder menu's Sequence JPEG Timestamps dialog with progress and cancel ([#1041](https://github.com/minodisk/handpic/issues/1041)) ([abd0522](https://github.com/minodisk/handpic/commit/abd05221ddceb6e009545f97adca4c0ce988a61b))
* **native:** add the folder tree pane and open folders from it ([#1014](https://github.com/minodisk/handpic/issues/1014)) ([71d5dc5](https://github.com/minodisk/handpic/commit/71d5dc528aeee38ec0f7db29f9a5e984d34db662))
* **native:** add the folder tree's keyboard, Expand All and Collapse All ([#1025](https://github.com/minodisk/handpic/issues/1025)) ([9bd7856](https://github.com/minodisk/handpic/commit/9bd7856600808548f76a9ccb82a581a768837e4b))
* **native:** add the Import tab to the settings modal ([#1033](https://github.com/minodisk/handpic/issues/1033)) ([f47bf51](https://github.com/minodisk/handpic/commit/f47bf51505ecb2f5d14b3fdeb18b3d7ed775cf32))
* **native:** add the meta pane and the view-only JPEG folder ([#975](https://github.com/minodisk/handpic/issues/975)) ([060e21d](https://github.com/minodisk/handpic/commit/060e21d58c53c997cc3aaaecf14f8d2ee0ec6073))
* **native:** add the performance overlay on d ([#1021](https://github.com/minodisk/handpic/issues/1021)) ([d46c922](https://github.com/minodisk/handpic/commit/d46c9221f10727987d2e7b5ab50c575aae0c1b4f))
* **native:** add the settings modal shell and the Culling tab ([#1022](https://github.com/minodisk/handpic/issues/1022)) ([7a8dbef](https://github.com/minodisk/handpic/commit/7a8dbef733f27a78a55d27477525cbbc73d2bd93))
* **native:** add the sort and filter menus to the status row ([#993](https://github.com/minodisk/handpic/issues/993)) ([40e3ec5](https://github.com/minodisk/handpic/commit/40e3ec5700812947e53e5a5360fe8dc888b91a81))
* **native:** add the strip's multi-selection and judge the whole selection ([#1028](https://github.com/minodisk/handpic/issues/1028)) ([6e0ce1d](https://github.com/minodisk/handpic/commit/6e0ce1db68b88c4bb64160ea938c394a5854a8e9))
* **native:** add the strip's right-click menu ([#1030](https://github.com/minodisk/handpic/issues/1030)) ([004dda8](https://github.com/minodisk/handpic/commit/004dda8540ce422d3abf1fd5c8daa5b350c72a2b))
* **native:** add the thumbnail strip in sync with the preview ([#964](https://github.com/minodisk/handpic/issues/964)) ([3e3893c](https://github.com/minodisk/handpic/commit/3e3893cfa7e86ed9a4ff4393e10f94cafd26b10c))
* **native:** call gpui_component::init and map the theme to Neutral dark ([#1056](https://github.com/minodisk/handpic/issues/1056)) ([15b26e3](https://github.com/minodisk/handpic/commit/15b26e30c6fbf9d3ac1d2868cfe896cec4937d67))
* **native:** draw a stale crop before chasing the current one ([#1013](https://github.com/minodisk/handpic/issues/1013)) ([26e1f7f](https://github.com/minodisk/handpic/commit/26e1f7fb9a79b1784f4d426f07fa82a136fec050))
* **native:** draw a stale preview before chasing the current file ([#1010](https://github.com/minodisk/handpic/issues/1010)) ([6680e48](https://github.com/minodisk/handpic/commit/6680e4853b37f7d83085bbb941341d3b21a2016f))
* **native:** draw the f focus mark colored by the stored analysis ([#974](https://github.com/minodisk/handpic/issues/974)) ([90ce921](https://github.com/minodisk/handpic/commit/90ce921323d4ffa91c037b072260afb12c1c88e3))
* **native:** draw the face boxes with their landmark dots and labels ([#977](https://github.com/minodisk/handpic/issues/977)) ([8d2b446](https://github.com/minodisk/handpic/commit/8d2b44624f5f79e75f41e4879ee65d28e40036bc))
* **native:** draw the filter menu's icons by kind and head every section ([#1057](https://github.com/minodisk/handpic/issues/1057)) ([22d6e6a](https://github.com/minodisk/handpic/commit/22d6e6a0bc2e5dad1c258d7ee750dda32bdc0769))
* **native:** draw the settings modal's controls with gpui-component ([#1061](https://github.com/minodisk/handpic/issues/1061)) ([26e4e15](https://github.com/minodisk/handpic/commit/26e4e15d066130d688f937609ec8fcf136c21e82))
* **native:** draw the sort, filter and right-click menus with the theme ([#1062](https://github.com/minodisk/handpic/issues/1062)) ([2595b1b](https://github.com/minodisk/handpic/commit/2595b1bb2fa10afec6726bc5fd0fa810d5403069))
* **native:** edit the keymap live from the Keyboard Shortcuts tab ([#1029](https://github.com/minodisk/handpic/issues/1029)) ([588e557](https://github.com/minodisk/handpic/commit/588e5573dae89a0a275fdb81089261cf1bbf8da2))
* **native:** fill the Face focus, Eyes and Output filter sections ([#992](https://github.com/minodisk/handpic/issues/992)) ([09fd508](https://github.com/minodisk/handpic/commit/09fd508489587b952263165aa8870e60e668842d))
* **native:** filter the strip by the stored judgment sections ([#991](https://github.com/minodisk/handpic/issues/991)) ([eb462f0](https://github.com/minodisk/handpic/commit/eb462f08b7710b9c1c57d2723559dfe8f21a419c))
* **native:** follow the disk in the folder tree and add its right-click menu ([#1019](https://github.com/minodisk/handpic/issues/1019)) ([4f823d9](https://github.com/minodisk/handpic/commit/4f823d96d541f4526af7f23c2fdfd40b5c1f1ef6))
* **native:** give the native app its icon on Windows, macOS and Linux ([#1058](https://github.com/minodisk/handpic/issues/1058)) ([b8fad0f](https://github.com/minodisk/handpic/commit/b8fad0f848db70da8897c8757e45a3d3b87f10e1))
* **native:** group bursts, draw their bands and badges and add the burst moves ([#1047](https://github.com/minodisk/handpic/issues/1047)) ([6f1e793](https://github.com/minodisk/handpic/commit/6f1e793611989eaf19b0ae4793c2e8fef0825d8b))
* **native:** hold the decoded neighbours in a prefetch ring ([#1026](https://github.com/minodisk/handpic/issues/1026)) ([8caadd0](https://github.com/minodisk/handpic/commit/8caadd054a98efa118f035f076391fc97abb005d))
* **native:** judge the eyes on demand and draw the face mesh ([#978](https://github.com/minodisk/handpic/issues/978)) ([397ad73](https://github.com/minodisk/handpic/commit/397ad733aece8c48bea9a4745b41e73ee8f2eae8))
* **native:** keep the settings in a SQLite store seeded once from settings.json ([#1008](https://github.com/minodisk/handpic/issues/1008)) ([394e1c2](https://github.com/minodisk/handpic/commit/394e1c20039281c02c55c8d12bbace30db328f85))
* **native:** open a folder dropped onto the window ([#1052](https://github.com/minodisk/handpic/issues/1052)) ([54f7ab6](https://github.com/minodisk/handpic/commit/54f7ab6609fde181b8fd53231349d23fa2f11230))
* **native:** open the index and run pass 1 of the scan ([#963](https://github.com/minodisk/handpic/issues/963)) ([1770ffc](https://github.com/minodisk/handpic/commit/1770ffcb305aa7d587ffef076e015d4152ac2d51))
* **native:** order the strip by the stored sort key ([#990](https://github.com/minodisk/handpic/issues/990)) ([6b0fa0e](https://github.com/minodisk/handpic/commit/6b0fa0e8ab5dae06153063b58cbaafdaf03b6ab4))
* **native:** page a folder's RAW previews with stage timings ([#950](https://github.com/minodisk/handpic/issues/950)) ([fb41adb](https://github.com/minodisk/handpic/commit/fb41adb7b189edca08a38c42f0b615fa8d938463))
* **native:** persist the strip's filter and sort order in the settings store ([#1012](https://github.com/minodisk/handpic/issues/1012)) ([e341ef2](https://github.com/minodisk/handpic/commit/e341ef21c9c32a2e3ca3b189c124f1fb08c6f6e3))
* **native:** port the per-folder EXIF filter sections ([#1009](https://github.com/minodisk/handpic/issues/1009)) ([76b7e52](https://github.com/minodisk/handpic/commit/76b7e52281bbec5bb06f594031a64cb38d9d85c1))
* **native:** read the settings, the keymap and the sidecars' judgments ([#980](https://github.com/minodisk/handpic/issues/980)) ([5fda400](https://github.com/minodisk/handpic/commit/5fda400345185825c6c2245c30000fd4dccad886))
* **native:** record judgments from the MCP companion's set_judgment ([#1020](https://github.com/minodisk/handpic/issues/1020)) ([4862eee](https://github.com/minodisk/handpic/commit/4862eee29154428ea1f0e4c420964c52e08f8da3))
* **native:** replace textinput.rs with gpui-component's Input ([#1059](https://github.com/minodisk/handpic/issues/1059)) ([6ff22c4](https://github.com/minodisk/handpic/commit/6ff22c4c1da26ac08cee924d003fcf70ea86a1fe))
* **native:** restore the window geometry and the last folder on launch ([#1040](https://github.com/minodisk/handpic/issues/1040)) ([88152a9](https://github.com/minodisk/handpic/commit/88152a910ddc0b41fa706a8a3baf605b10437ea7))
* **native:** resync the open folder from the folder watcher ([#966](https://github.com/minodisk/handpic/issues/966)) ([7a5eaf5](https://github.com/minodisk/handpic/commit/7a5eaf5f4f35597f2d4fc3436393dbd5b18ec271))
* **native:** run Open Folder and the pane toggles and remember the panes ([#1042](https://github.com/minodisk/handpic/issues/1042)) ([1ce45ab](https://github.com/minodisk/handpic/commit/1ce45ab40c3e76652f75aada17f1935166eff3ea))
* **native:** run pass 2 after pass 1 on the scan thread ([#972](https://github.com/minodisk/handpic/issues/972)) ([1bfc54a](https://github.com/minodisk/handpic/commit/1bfc54a68134574df3beb9547a57c0f8bc032431))
* **native:** scaffold the GPUI app as its own workspace ([#944](https://github.com/minodisk/handpic/issues/944)) ([ccb8a02](https://github.com/minodisk/handpic/commit/ccb8a0259f8f2b8f61a1814d8741783e57d682eb))
* **native:** serve the MCP endpoint with the read and navigate tools ([#1015](https://github.com/minodisk/handpic/issues/1015)) ([f4ba9b9](https://github.com/minodisk/handpic/commit/f4ba9b933511eaaea80b8d24af77872e8bb46b20))
* **native:** show a progress bar for the scan and Move to Trash ([#1053](https://github.com/minodisk/handpic/issues/1053)) ([ec50782](https://github.com/minodisk/handpic/commit/ec50782bcc90640bc4a817df529c210534b4cdfe))
* **native:** show the empty state in the preview area ([#1048](https://github.com/minodisk/handpic/issues/1048)) ([83cbd56](https://github.com/minodisk/handpic/commit/83cbd56748ba1fb8fb95fcae7b9bba675247432d))
* **native:** steer the scan from the strip and resume at the last viewed file ([#965](https://github.com/minodisk/handpic/issues/965)) ([7e5c8d7](https://github.com/minodisk/handpic/commit/7e5c8d7771713f5412b2205ebbab2ef30f8be35d))
* **native:** switch the sidecar format live from the Sidecar tab ([#1027](https://github.com/minodisk/handpic/issues/1027)) ([0e5fe76](https://github.com/minodisk/handpic/commit/0e5fe766893bd90f0dba351b3b5852a560c95243))
* **native:** undo and redo a judgment ([#983](https://github.com/minodisk/handpic/issues/983)) ([923d852](https://github.com/minodisk/handpic/commit/923d8526a6a63850a40588e0c527337f4935ce32))
* **native:** undo and redo a move to the Trash ([#1035](https://github.com/minodisk/handpic/issues/1035)) ([4e018d8](https://github.com/minodisk/handpic/commit/4e018d895d0883eeefceeb9011cca6bca8d21a5a))
* **native:** write Handpic.log from the GPUI app ([#998](https://github.com/minodisk/handpic/issues/998)) ([0b175c0](https://github.com/minodisk/handpic/commit/0b175c0979ce3a4bccb789af7cc21c0526535ce1))
* **native:** write the judgments to the index and the sidecars ([#981](https://github.com/minodisk/handpic/issues/981)) ([06d673b](https://github.com/minodisk/handpic/commit/06d673bcbd681dc4b5dd49d621058cb88e73636e))


### Bug Fixes

* **app:** iterate the prefetch ring's sets without copying ([#951](https://github.com/minodisk/handpic/issues/951)) ([a91a267](https://github.com/minodisk/handpic/commit/a91a2678b8aca794cb7cb7001fcaed78864d9503))
* **native:** reset the scan status on a stop and stop the scan on quit ([#988](https://github.com/minodisk/handpic/issues/988)) ([244690d](https://github.com/minodisk/handpic/commit/244690df381850ec2f43e52914e58df6cc80b115))


### Performance Improvements

* **scan:** keep paging fast while pass 2 runs ([#984](https://github.com/minodisk/handpic/issues/984)) ([2b2adcf](https://github.com/minodisk/handpic/commit/2b2adcfb0b1678228cd2b3b90a4093930e75dd22))

## [1.10.0](https://github.com/minodisk/riffle/compare/v1.9.0...v1.10.0) (2026-11-07)


### Features

* **app:** add a ? that defines the Face focus items ([#923](https://github.com/minodisk/riffle/issues/923)) ([565b584](https://github.com/minodisk/riffle/commit/565b584dde734a63a0bc43233c7d530ad780aea4))
* **app:** add Delete Permanently to the strip's menu (trash-over-scan Step 5) ([#918](https://github.com/minodisk/riffle/issues/918)) ([34fad12](https://github.com/minodisk/riffle/commit/34fad120b79225a04fb8967aa3d6a3cf181c89d4))
* **app:** add the import destination root and folder pattern settings ([#894](https://github.com/minodisk/riffle/issues/894)) ([cd0d023](https://github.com/minodisk/riffle/commit/cd0d023f0aafd7e608ced28baf03d7a3df06b1ed))
* **app:** add the import plan and the import_preview command (import-from-card Step 3) ([#900](https://github.com/minodisk/riffle/issues/900)) ([b986ab4](https://github.com/minodisk/riffle/commit/b986ab4d93b72ecac16a965793228a31d96e3102))
* **app:** add the import run, its dialog and the tree's Import item (import-from-card Step 4) ([#902](https://github.com/minodisk/riffle/issues/902)) ([90988af](https://github.com/minodisk/riffle/commit/90988af977bc0fb75f313e470532c67715d7f943))
* **app:** add the outputs_of command that finds a folder's exported files ([#912](https://github.com/minodisk/riffle/issues/912)) ([d74c215](https://github.com/minodisk/riffle/commit/d74c215ac287822a4fbfc2ad99cb013dc544de07))
* **app:** add the Rename by Capture Date item and its confirmation dialog ([#895](https://github.com/minodisk/riffle/issues/895)) ([96cc29a](https://github.com/minodisk/riffle/commit/96cc29a5b3b2c6cad2348c8a7ccc2b5a7d575254))
* **app:** cancel the running scan when Move to Trash is held (trash-over-scan Step 1) ([#910](https://github.com/minodisk/riffle/issues/910)) ([b0da6a1](https://github.com/minodisk/riffle/commit/b0da6a1051701cbb2628a4cffc9607eb928872fb))
* **app:** cull while importing by opening the first date folder (import-from-card Step 5) ([#906](https://github.com/minodisk/riffle/issues/906)) ([c233f60](https://github.com/minodisk/riffle/commit/c233f60f24a1ac23768ecdc709db413c08c1abd2))
* **app:** filter the strip by whether a file has an export (filter-by-output Step 2) ([#914](https://github.com/minodisk/riffle/issues/914)) ([eecb48b](https://github.com/minodisk/riffle/commit/eecb48b3667ed240d7f10808bbee12217460686d))
* **app:** move a file's index rows across folders and forget a folder ([#891](https://github.com/minodisk/riffle/issues/891)) ([fec6c72](https://github.com/minodisk/riffle/commit/fec6c72c940b064167aa4d665785595c5ee53fb6))
* **app:** move files to the Trash in chunks on Windows (trash-over-scan Step 4) ([#917](https://github.com/minodisk/riffle/issues/917)) ([c2e70af](https://github.com/minodisk/riffle/commit/c2e70af25485c88ec8e093cb2fa104693d5367c8))
* **app:** plan a folder's rename or split by capture date ([#889](https://github.com/minodisk/riffle/issues/889)) ([dc1db61](https://github.com/minodisk/riffle/commit/dc1db6185ccccb3c6447e9731b100f54f38e242c))
* **app:** rename or split a folder by capture date with date_folder_run ([#893](https://github.com/minodisk/riffle/issues/893)) ([5d9578d](https://github.com/minodisk/riffle/commit/5d9578d6442aa3f0ee59e9566c8b22ddaa242d28))
* **app:** split the filter's AF eye section into Face focus Good / Neutral / Low ([#922](https://github.com/minodisk/riffle/issues/922)) ([8203ce7](https://github.com/minodisk/riffle/commit/8203ce73d5c564c36dc8fe076a8668062e36b5a1))
* **app:** stream Move to Trash progress and refuse a second move (trash-over-scan Step 2) ([#913](https://github.com/minodisk/riffle/issues/913)) ([5a1ca35](https://github.com/minodisk/riffle/commit/5a1ca358d91a4888da4d80e1de7ee3fb11f5f5f9))
* **cli:** dump the mesh landmarks, offsets and window score in facechoice (ef-gate-fill Step 2) ([#899](https://github.com/minodisk/riffle/issues/899)) ([5265796](https://github.com/minodisk/riffle/commit/52657961f25bcc170accec4ed0a16890887dfa4c))
* **core:** add the import copy pipeline and a copybench CLI ([#898](https://github.com/minodisk/riffle/issues/898)) ([a188806](https://github.com/minodisk/riffle/commit/a1888062bb7a14d4a3417b443833be8938cd8001))
* **plans:** add gatescores.py and fix the ef-gate tau grid (ef-gate-fill Step 3) ([#901](https://github.com/minodisk/riffle/issues/901)) ([6746615](https://github.com/minodisk/riffle/commit/6746615d6c582b137e4857e225ce63fe7b4623c0))
* **plans:** read the ef-gate-fill exploration, the stop rule ends it (ef-gate-fill Step 4) ([#903](https://github.com/minodisk/riffle/issues/903)) ([2882130](https://github.com/minodisk/riffle/commit/2882130696d714850ecaf6682143b4c67fc4a93c))


### Bug Fixes

* **app:** keep scan_folder off the runtime workers while a move holds Scans (trash-over-scan Step 3) ([#916](https://github.com/minodisk/riffle/issues/916)) ([674cf12](https://github.com/minodisk/riffle/commit/674cf12d5fc38adc35c37ac22e16365a0871ccba))

## [1.9.0](https://github.com/minodisk/riffle/compare/v1.8.0...v1.9.0) (2026-11-03)


### Features

* **app:** advance after un-reject / un-pick when Auto-advance is on ([#885](https://github.com/minodisk/riffle/issues/885)) ([cd04328](https://github.com/minodisk/riffle/commit/cd04328fc9a014919f0f31ca36b890b9f28a724d))
* **app:** show yaw, pitch and roll as separate meta rows ([#861](https://github.com/minodisk/riffle/issues/861)) ([8ec2083](https://github.com/minodisk/riffle/commit/8ec20833edece3472899f02a01aebccac43193a2))
* **app:** show YuNet's score, size, eye distance and five landmarks beside each face box ([#867](https://github.com/minodisk/riffle/issues/867)) ([22c694c](https://github.com/minodisk/riffle/commit/22c694c4bb134e8880d56198cb27080e09eafc65))
* **cli:** add the facechoice per-face eye_focus dump (Step 2) ([#872](https://github.com/minodisk/riffle/issues/872)) ([7abe15e](https://github.com/minodisk/riffle/commit/7abe15ea528b92599120603ca0a3af46a4485812))
* **plans:** dump the six fresh folders label-free (Step 4) ([#880](https://github.com/minodisk/riffle/issues/880)) ([ef96e33](https://github.com/minodisk/riffle/commit/ef96e33d802645c56299ddeb9a0492c431597231))
* **plans:** fit every model-search candidate on both fit sets (Step 2) ([#863](https://github.com/minodisk/riffle/issues/863)) ([ca553e1](https://github.com/minodisk/riffle/commit/ca553e11c57c91cb8f643ca2bde8bb5826b19229))
* **plans:** fit the discard candidates out of fold (Step 2) ([#878](https://github.com/minodisk/riffle/issues/878)) ([fd6c976](https://github.com/minodisk/riffle/commit/fd6c9767bb7d872b20e3e5f923b20dc4dd034be6))
* **plans:** fit the held-out G0+B, D0 and D0+B scores on every dump frame (Step 3) ([#855](https://github.com/minodisk/riffle/issues/855)) ([825b2ac](https://github.com/minodisk/riffle/commit/825b2aca6bba62049c843d356b80056cd633428b))
* **plans:** pre-register the confirmation and draw the fresh blind sample (Step 5) ([#881](https://github.com/minodisk/riffle/issues/881)) ([5896103](https://github.com/minodisk/riffle/commit/58961031cc254c4c3c4de8f6926d016d6f99f189))
* **plans:** pre-register the model-search confirmation and draw the fresh blind sample (Step 4) ([#865](https://github.com/minodisk/riffle/issues/865)) ([d429ac8](https://github.com/minodisk/riffle/commit/d429ac8dffd8fd262ca3a9ff7312a987ad1eaed4))
* **plans:** read the discard candidates on both rounds, LR/T wins (Step 3) ([#879](https://github.com/minodisk/riffle/issues/879)) ([5b93ca6](https://github.com/minodisk/riffle/commit/5b93ca61d4a6a8c69f132dfdade24bc4e7256245))
* **plans:** read the discard-retarget cells against the blind labels (Step 4) ([#857](https://github.com/minodisk/riffle/issues/857)) ([53252ec](https://github.com/minodisk/riffle/commit/53252ecfd802e53d0062a5665fd139b74e155e7b))
* **plans:** read the face-choice rules once (Step 3) ([#873](https://github.com/minodisk/riffle/issues/873)) ([94dadc7](https://github.com/minodisk/riffle/commit/94dadc714ff2a1452f0047a0b5ffec940789aba2))
* **plans:** read the fresh blind confirmation (Step 6) ([#882](https://github.com/minodisk/riffle/issues/882)) ([3c8f748](https://github.com/minodisk/riffle/commit/3c8f74823b0bf92f0d1534bea7e525b923a821ba))
* **plans:** read the model-search confirmation once (Step 5) ([#866](https://github.com/minodisk/riffle/issues/866)) ([b032d0a](https://github.com/minodisk/riffle/commit/b032d0aebbba8351afba2f8122537f0a06650f9f))
* **plans:** read the model-search exploration and choose the winners (Step 3) ([#864](https://github.com/minodisk/riffle/issues/864)) ([9d57d9f](https://github.com/minodisk/riffle/commit/9d57d9f95e358488a3009c1b6b728cf565fdb1ff))
* **plans:** write the model-search Decision (Step 6) ([#868](https://github.com/minodisk/riffle/issues/868)) ([d444caf](https://github.com/minodisk/riffle/commit/d444caf7beb78c7c635595e491a97b2412ec43fd))

## [1.8.0](https://github.com/minodisk/riffle/compare/v1.7.0...v1.8.0) (2026-10-20)


### Features

* **app:** count a file as viewed only after a 500 ms dwell ([#813](https://github.com/minodisk/riffle/issues/813)) ([97453c8](https://github.com/minodisk/riffle/commit/97453c8e4448eff9c7d17d782fec1c4ec98017b2))
* **app:** fit the ranking on per-file views instead of Culling Finished ([#806](https://github.com/minodisk/riffle/issues/806)) ([9083383](https://github.com/minodisk/riffle/commit/908338306604931da00ff497f632c38fc1869a6f))
* **app:** mark the burst's first-ranked frame with an icon on the strip ([#805](https://github.com/minodisk/riffle/issues/805)) ([ab278ae](https://github.com/minodisk/riffle/commit/ab278ae61fe6fda7d45ede5e908b2d1f5add3167))
* **app:** refit the ranking at startup and on a folder switch ([#810](https://github.com/minodisk/riffle/issues/810)) ([f924609](https://github.com/minodisk/riffle/commit/f924609c17f7510222d5e92a83cc6ac254c62002))
* **app:** reveal a strip file and copy its path or name from the right-click menu ([#844](https://github.com/minodisk/riffle/issues/844)) ([47b072b](https://github.com/minodisk/riffle/commit/47b072b8242e178e9696ba636a6af316bd473b6d))
* **app:** show the genre a burst was ranked as in the meta pane ([#817](https://github.com/minodisk/riffle/issues/817)) ([e12286b](https://github.com/minodisk/riffle/commit/e12286bd2801fea8c8285b6af85f33b4cb89d4f8))
* **core:** learn and rank with per-genre offsets on the shared ranking weights ([#816](https://github.com/minodisk/riffle/issues/816)) ([4dcc2b2](https://github.com/minodisk/riffle/commit/4dcc2b2e4a692c3c5bbca74024b9111ee941e30b))
* **plans:** add the bad-photo label sampler and labeling guide (Step 1) ([#838](https://github.com/minodisk/riffle/issues/838)) ([1e9215d](https://github.com/minodisk/riffle/commit/1e9215df9032a4028c5a0b4436106226dd5f5df0))
* **plans:** compute the burst-relative features on every dump frame (Step 1) ([#853](https://github.com/minodisk/riffle/issues/853)) ([1cf277d](https://github.com/minodisk/riffle/commit/1cf277d119fe346b1d45f69626d14944473e0e84))
* **plans:** draw the blind discard-review sample of G0 candidates and controls (Step 1) ([#846](https://github.com/minodisk/riffle/issues/846)) ([b4c9584](https://github.com/minodisk/riffle/commit/b4c95842bc1d7c24d4e270759f6bd108c508a6c4))
* **plans:** fit the dedicated bad-photo models B0 / L0 and record the Decision (Step 5) ([#842](https://github.com/minodisk/riffle/issues/842)) ([df1071a](https://github.com/minodisk/riffle/commit/df1071a48b46ea6df9ab4b154b4ddad2b0145ad7))
* **plans:** measure every score, rule and cue against the bad-photo labels (Step 4) ([#841](https://github.com/minodisk/riffle/issues/841)) ([1bc10e0](https://github.com/minodisk/riffle/commit/1bc10e053a6ee97f0c913ae7274ad432736866d7))
* **plans:** measure the discard cuts against the blind labels (Step 4) ([#849](https://github.com/minodisk/riffle/issues/849)) ([ed10472](https://github.com/minodisk/riffle/commit/ed104725c914d6aae3769faff4099e4b3cff51b3))
* **plans:** read the user's bad-photo labels into labels.tsv (Step 3) ([#840](https://github.com/minodisk/riffle/issues/840)) ([a5d4150](https://github.com/minodisk/riffle/commit/a5d4150b741ceffe7bc643682e608508ef72f848))
* **plans:** read the user's discard labels into labels.tsv (Step 3) ([#848](https://github.com/minodisk/riffle/issues/848)) ([36bdb35](https://github.com/minodisk/riffle/commit/36bdb3506da4400c02b65a1e4ec3e39e6cd1b12d))


### Bug Fixes

* **app:** show the written count and capture-time changes in the sequence preview ([#821](https://github.com/minodisk/riffle/issues/821)) ([8b2f788](https://github.com/minodisk/riffle/commit/8b2f788d80b519e434183cd932e6684c7b28d611))

## [1.7.0](https://github.com/minodisk/riffle/compare/v1.6.0...v1.7.0) (2026-10-13)


### Features

* **app:** add Move to Trash to the filmstrip's right-click menu ([#797](https://github.com/minodisk/riffle/issues/797)) ([8ee27ce](https://github.com/minodisk/riffle/commit/8ee27ce38d3655e08dcd64e59ee49b42014b3974))
* **app:** add the trash_files command ([#795](https://github.com/minodisk/riffle/issues/795)) ([d692a15](https://github.com/minodisk/riffle/commit/d692a158bb2a4e22902b5c53fa3a6ed451307168))
* **app:** mark a folder Culling Finished once every file has been previewed ([#800](https://github.com/minodisk/riffle/issues/800)) ([3a9b9f0](https://github.com/minodisk/riffle/commit/3a9b9f0dd98db3403e5e0d4e4a5a779b4122ae4e))
* **app:** rank each burst's frames by a personal model behind aiEnabled ([#794](https://github.com/minodisk/riffle/issues/794)) ([0a994e8](https://github.com/minodisk/riffle/commit/0a994e857365962c9b975cac7645ad59cd20960a))
* **app:** show disk read and disk busy series in the performance overlay ([#791](https://github.com/minodisk/riffle/issues/791)) ([e47b000](https://github.com/minodisk/riffle/commit/e47b0009db82d82af6de74736b285f15060064ab))

## [1.6.0](https://github.com/minodisk/riffle/compare/v1.5.0...v1.6.0) (2026-10-12)


### Features

* **app:** add a fair tier below good with its strip icon and filter items (Step 4c) ([#751](https://github.com/minodisk/riffle/issues/751)) ([f5dfef9](https://github.com/minodisk/riffle/commit/f5dfef927d476aa0168889bfdf44137f77b89564))
* **app:** add a Genre filter section from the image embeddings ([#789](https://github.com/minodisk/riffle/issues/789)) ([73e277b](https://github.com/minodisk/riffle/commit/73e277b6660324611673bd30d66480c2a5e649bd))
* **app:** add filter / set_filter commands that store a validated strip filter ([#770](https://github.com/minodisk/riffle/issues/770)) ([6e2b180](https://github.com/minodisk/riffle/commit/6e2b18010350c4599f54c16d51a1673fdd953c93))
* **app:** add the perf-sample CPU sampler behind set_perf_sampling ([#778](https://github.com/minodisk/riffle/issues/778)) ([38e1180](https://github.com/minodisk/riffle/commit/38e1180fdd4c8fe28ea30e078c8f73fdaef36d42))
* **app:** add the performance graph overlay toggled by the performance action ([#780](https://github.com/minodisk/riffle/issues/780)) ([6419de0](https://github.com/minodisk/riffle/commit/6419de0e34acc67f50e80c033159758fd8bb56ea))
* **app:** add the performance overlay's latency model and record per-switch latency ([#779](https://github.com/minodisk/riffle/issues/779)) ([7e9a733](https://github.com/minodisk/riffle/commit/7e9a7334016b186b22efd633de22a9f64db2182a))
* **app:** advance after a repeated star, reject or pick ([#790](https://github.com/minodisk/riffle/issues/790)) ([17da52d](https://github.com/minodisk/riffle/commit/17da52decf4bb81732d6ec88e49a6ba54301bc8b))
* **app:** draw the face mesh over the judged face ([#718](https://github.com/minodisk/riffle/issues/718)) ([cbe8f02](https://github.com/minodisk/riffle/commit/cbe8f0239dc5c5219b143d5544e9f913646ad020))
* **app:** draw the face parts outline and irises instead of the mesh ([#722](https://github.com/minodisk/riffle/issues/722)) ([bde2366](https://github.com/minodisk/riffle/commit/bde23660efa8fc83942b9c80164e76a48f138383))
* **app:** embed each RAW file in a third scan pass behind the aiEnabled setting ([#787](https://github.com/minodisk/riffle/issues/787)) ([0e28a11](https://github.com/minodisk/riffle/commit/0e28a11e02d21d58ca761c8f80f617b3da28dca4))
* **app:** exclude mesh misfits and edge-cut faces from the photo tiers and tighten yaw to 30 (Step 5) ([#752](https://github.com/minodisk/riffle/issues/752)) ([f853329](https://github.com/minodisk/riffle/commit/f8533295f3dbfc2d31ba3efa29dc32cb01a24f18))
* **app:** expose the shown file's eyes judgment and head pose in get_view ([#739](https://github.com/minodisk/riffle/issues/739)) ([4cfd319](https://github.com/minodisk/riffle/commit/4cfd319aa0628acebb9307d94af60aedca7dea50))
* **app:** fold the good and fair tiers into one good tier (Step 5b) ([#754](https://github.com/minodisk/riffle/issues/754)) ([054d1a1](https://github.com/minodisk/riffle/commit/054d1a1d03a28cac76a2891210ff829e457a537d))
* **app:** judge a frame with no trusted AF point on the largest confident face of the whole preview ([#768](https://github.com/minodisk/riffle/issues/768)) ([a338845](https://github.com/minodisk/riffle/commit/a33884595f63ffa35c9a7e5a2046e60c8cfe3be3))
* **app:** judge the shown file's eyes on demand in the meta pane ([#712](https://github.com/minodisk/riffle/issues/712)) ([392f9c2](https://github.com/minodisk/riffle/commit/392f9c257db8030f4b5737c17a2ab3dfbfeb84a6))
* **app:** mark only good photos with the strip icon and the bright green crosshair ([#748](https://github.com/minodisk/riffle/issues/748)) ([ccd18aa](https://github.com/minodisk/riffle/commit/ccd18aaa96bbd153cee9c7566736ae9bb9a41a59))
* **app:** merge the AF eye OK state into Bad so the judgment is Good / Bad / Unknown ([#766](https://github.com/minodisk/riffle/issues/766)) ([da43e85](https://github.com/minodisk/riffle/commit/da43e8572e1a9500d46e1ee58a431f798de839da))
* **app:** narrow the AF eye filter's Sharp item to in focus but not Good ([#756](https://github.com/minodisk/riffle/issues/756)) ([bf9e9d1](https://github.com/minodisk/riffle/commit/bf9e9d101eb1da44c78b2936e30d74b1db1d9e25))
* **app:** re-tune the good-photo cuts on the 180 rated frames (Step 7) ([#761](https://github.com/minodisk/riffle/issues/761)) ([3e1b1a6](https://github.com/minodisk/riffle/commit/3e1b1a637bb6ce122a80b6577f515eb6a809b42a))
* **app:** remember the strip's filter across restarts ([#772](https://github.com/minodisk/riffle/issues/772)) ([9b5f0c5](https://github.com/minodisk/riffle/commit/9b5f0c5e129dc1727b3ae4e5759f1fc9214632e6))
* **app:** remove the sharpness bar, the compare BEST mark and the Sharpness row ([#775](https://github.com/minodisk/riffle/issues/775)) ([49d78a0](https://github.com/minodisk/riffle/commit/49d78a0eab9b6c547494d22033a5d9caea239bae))
* **app:** rename the AF eye items to Good / OK / Bad and give each state its own icon and mark color ([#762](https://github.com/minodisk/riffle/issues/762)) ([22f01d7](https://github.com/minodisk/riffle/commit/22f01d7436d9d2a350c7898fa52454f3c6ff66ad))
* **app:** return the face mesh points from eyes_of ([#717](https://github.com/minodisk/riffle/issues/717)) ([c1ad78d](https://github.com/minodisk/riffle/commit/c1ad78d0d76e1e396d8374abae80f14ea441ab99))
* **app:** show the eyes as openness from the EAR in the meta pane (Step 4b) ([#750](https://github.com/minodisk/riffle/issues/750)) ([a0de444](https://github.com/minodisk/riffle/commit/a0de444152e6ccf1fa0ae52dcbf4a5fe133bf238))
* **app:** show the Eyes row as the open probability ([#721](https://github.com/minodisk/riffle/issues/721)) ([d74d762](https://github.com/minodisk/riffle/commit/d74d762522a969105eae5171a14c9aa969b252e7))
* **app:** show the head pose of the judged face in the meta pane ([#725](https://github.com/minodisk/riffle/issues/725)) ([9048e95](https://github.com/minodisk/riffle/commit/9048e952623b0d766c498b29f2f590a619644dba))
* **app:** show the stored eyes and pose in the meta pane and add the Eyes filter ([#743](https://github.com/minodisk/riffle/issues/743)) ([65c70dd](https://github.com/minodisk/riffle/commit/65c70dd0bf93433a2d565df7acfbcee9051f754d))
* **app:** store the AF face's eye EAR and head pose in the index ([#741](https://github.com/minodisk/riffle/issues/741)) ([b0fc2cf](https://github.com/minodisk/riffle/commit/b0fc2cfac0bfc7592c178d2c339842371d9a76a9))
* **cli:** add the embed prototype and measure the candidate embedding models ([#785](https://github.com/minodisk/riffle/issues/785)) ([3aca8df](https://github.com/minodisk/riffle/commit/3aca8df097709f5eee1aec732f543eda559c1f43))
* **cli:** add the roll-corrected face mesh as opt-in functions and a meshfit dump (Step 1) ([#755](https://github.com/minodisk/riffle/issues/755)) ([a5ee087](https://github.com/minodisk/riffle/commit/a5ee087ea4752341a1ecf97b183fc72f619e9593))
* **cli:** dump the burst keep-score features per file ([#732](https://github.com/minodisk/riffle/issues/732)) ([f7db5cd](https://github.com/minodisk/riffle/commit/f7db5cd3e4a28c1a3f23f4b07de7dae24db45af1))
* **cli:** measure per-eye focus over the face mesh eye regions ([#729](https://github.com/minodisk/riffle/issues/729)) ([d509e55](https://github.com/minodisk/riffle/commit/d509e555d97b691d13e3d8e604d74872046d42da))
* **core:** add the eyelid contour mask primitives for the AF eye measures ([#742](https://github.com/minodisk/riffle/issues/742)) ([35b1edc](https://github.com/minodisk/riffle/commit/35b1edcefbff15cdde9cd17054fb2aaadd6cadbc))
* **core:** add the MediaPipe Face Landmarker v2 eyes module ([#710](https://github.com/minodisk/riffle/issues/710)) ([6d879cf](https://github.com/minodisk/riffle/commit/6d879cfb32a90e8e2410a5edfdfeecc850999983))
* **core:** port MediaPipe's face geometry for the head pose ([#723](https://github.com/minodisk/riffle/issues/723)) ([135e658](https://github.com/minodisk/riffle/commit/135e65803dc71a4adb5b7a033d5ce8542de58ebd))
* **core:** score the AF eye over the mesh eye regions in the scan ([#733](https://github.com/minodisk/riffle/issues/733)) ([a7d2fbc](https://github.com/minodisk/riffle/commit/a7d2fbc42806084cf324a8dae4e8bec3c0686b2d))
* **core:** skip the scan's face mesh for faces under 60 px ([#737](https://github.com/minodisk/riffle/issues/737)) ([ac6f6ad](https://github.com/minodisk/riffle/commit/ac6f6ade16236e975848b9cc34f3850734ddd5a3))


### Bug Fixes

* **app:** create the main window hidden so the restored geometry shows without a default-size flash ([#769](https://github.com/minodisk/riffle/issues/769)) ([aff7059](https://github.com/minodisk/riffle/commit/aff7059b64754f222c5795620d20aa9263e35df1))

## [1.5.0](https://github.com/minodisk/riffle/compare/v1.4.0...v1.5.0) (2026-10-07)


### Features

* **app:** log invariant violations in the scan lifecycle ([#706](https://github.com/minodisk/riffle/issues/706)) ([f54190f](https://github.com/minodisk/riffle/commit/f54190f9e525a2ebf8fd0d54cfa66d617d95fca5))
* **app:** log panics, uncaught JS errors and the remaining eprintln failures ([#705](https://github.com/minodisk/riffle/issues/705)) ([da352da](https://github.com/minodisk/riffle/commit/da352da8179f4a7c5e7032c7975958f8f057d5f3))
* **app:** look up PhotoLab Uuids on macOS ([#687](https://github.com/minodisk/riffle/issues/687)) ([ebbd099](https://github.com/minodisk/riffle/commit/ebbd0991a0a450f3a0443fa2329e2f7237471b02))
* **app:** reload the open folder from the tree's Refresh ([#692](https://github.com/minodisk/riffle/issues/692)) ([d03356d](https://github.com/minodisk/riffle/commit/d03356d7e0340f07ff60eb291bdfe2346ba08297))
* **app:** rewrite a folder's sidecars from the index ([#688](https://github.com/minodisk/riffle/issues/688)) ([6da10d2](https://github.com/minodisk/riffle/commit/6da10d28a86d95e674ca7ab65e30ce0cc26ec067))
* **app:** rewrite and delete a folder's sidecars ([#691](https://github.com/minodisk/riffle/issues/691)) ([deed9d6](https://github.com/minodisk/riffle/commit/deed9d662ceea988b53e647fddca678e3765756f))
* **app:** show a pending inline rename until the scan finishes ([#690](https://github.com/minodisk/riffle/issues/690)) ([07eaacc](https://github.com/minodisk/riffle/commit/07eaaccd6350bedd0267b9831f6ba709e9a00249))
* **app:** show a scan progress bar and pulse cells without a thumbnail ([#684](https://github.com/minodisk/riffle/issues/684)) ([9f812ba](https://github.com/minodisk/riffle/commit/9f812bace1813ae2146550c2a4063b8a80276ddc))
* **cli:** add a folder-level face detection report and record the baseline ([#693](https://github.com/minodisk/riffle/issues/693)) ([21c2db4](https://github.com/minodisk/riffle/commit/21c2db4493386069424cd7296e20bfddf43be85e))
* **cli:** survey eye-state models and adopt Face Landmarker v2 on demand ([#709](https://github.com/minodisk/riffle/issues/709)) ([e5b55ae](https://github.com/minodisk/riffle/commit/e5b55ae96c28980af3a2069bdd8f85b523b8acad))
* **core:** decode the preview at a DCT scale for the whole-image face search ([#698](https://github.com/minodisk/riffle/issues/698)) ([9724b1f](https://github.com/minodisk/riffle/commit/9724b1f8968fd57cca0f83d8bc98930750441f98))
* **core:** search the whole preview for faces at a 640x448 model input ([#696](https://github.com/minodisk/riffle/issues/696)) ([9ba5daf](https://github.com/minodisk/riffle/commit/9ba5daf63dde0df27cb6cdde98225ccb91d318fb))


### Bug Fixes

* **app:** refuse an inline rename while that rename is in flight ([#703](https://github.com/minodisk/riffle/issues/703)) ([8109183](https://github.com/minodisk/riffle/commit/81091832d02b8b7046165cc14fd02aada5468a8a))
* **app:** retry the PhotoLab Uuid lookup on a busy database ([#682](https://github.com/minodisk/riffle/issues/682)) ([b5291d4](https://github.com/minodisk/riffle/commit/b5291d422a5b139c2a1fa62b77b8025de44fe61b))
* **core:** fall back to the preview when an ARW has no larger JPEG ([#683](https://github.com/minodisk/riffle/issues/683)) ([b3cc725](https://github.com/minodisk/riffle/commit/b3cc7254f4cf7e47a6847fafd4a676b42266e303))


### Performance Improvements

* **app:** run log_timing off the main thread ([#697](https://github.com/minodisk/riffle/issues/697)) ([97b6318](https://github.com/minodisk/riffle/commit/97b631801254ca675cef0c95158e25c5cd07bcaa))

## [1.4.0](https://github.com/minodisk/riffle/compare/v1.3.0...v1.4.0) (2026-10-04)


### Features

* **app:** name the trigger of a deferred rescan in the timing log ([#670](https://github.com/minodisk/riffle/issues/670)) ([3139db9](https://github.com/minodisk/riffle/commit/3139db90e0474f53adf33d0f70b964e0bc43cf5c))
* **core:** add Lightroom color label presets for every Lightroom Classic UI language ([#677](https://github.com/minodisk/riffle/issues/677)) ([3c3fd4b](https://github.com/minodisk/riffle/commit/3c3fd4b47c0604463102e3131f3aee8e4387e8be))


### Bug Fixes

* **app:** defer the focus rescan while the folder picker is open ([#661](https://github.com/minodisk/riffle/issues/661)) ([18b449e](https://github.com/minodisk/riffle/commit/18b449e4bcf3aa0cbd528460511bcfa31048a706))
* **app:** drop the scan's running entry before emitting faces-done ([#665](https://github.com/minodisk/riffle/issues/665)) ([45be46c](https://github.com/minodisk/riffle/commit/45be46ca67008314f610ff0fa3c6340793a7de87))
* **app:** refuse to open a folder under a rename that is in flight ([#664](https://github.com/minodisk/riffle/issues/664)) ([9105cf8](https://github.com/minodisk/riffle/commit/9105cf82467ca915b3b274898e099a0ad74889f4))
* **claude:** make merger run its sync commands as separate, verbatim calls ([#671](https://github.com/minodisk/riffle/issues/671)) ([b9afac2](https://github.com/minodisk/riffle/commit/b9afac29e4a182e8b4a76f3ab0bdbc944e18aede))
* **claude:** make pr-runner run its scripts verbatim as standalone calls ([#676](https://github.com/minodisk/riffle/issues/676)) ([9460fef](https://github.com/minodisk/riffle/commit/9460fef52328f61a1e026b188a53bc1bd902095e))
* **claude:** make pr-runner wait for a child's hand-back instead of polling ([#666](https://github.com/minodisk/riffle/issues/666)) ([badc7b5](https://github.com/minodisk/riffle/commit/badc7b52d5be44272678082988efd3aa6199291f))
* **claude:** stop dropping read-only gh / gcloud entries in merge-settings ([#657](https://github.com/minodisk/riffle/issues/657)) ([971a9b9](https://github.com/minodisk/riffle/commit/971a9b96852649fe90ebef265a1f76558691ffd5))

## [1.3.0](https://github.com/minodisk/riffle/compare/v1.2.0...v1.3.0) (2026-10-02)


### Features

* **app:** add a Refresh item to the folder tree's right-click menu ([#652](https://github.com/minodisk/riffle/issues/652)) ([9e97b71](https://github.com/minodisk/riffle/commit/9e97b71134539713ecbf676ddc2706a90dc32f0c))
* **app:** keep the EXIF of a file whose extraction failed in its error row ([#650](https://github.com/minodisk/riffle/issues/650)) ([8394b20](https://github.com/minodisk/riffle/commit/8394b203d48cc976e5f2cb44a76e989ca8e99c98))
* **cli:** add a check subcommand that runs every sample through the app's stages ([#640](https://github.com/minodisk/riffle/issues/640)) ([cbeafbe](https://github.com/minodisk/riffle/commit/cbeafbee11a079f42d55439a331c22efca795519))


### Bug Fixes

* **app:** clear the main preview when the current file's preview fails ([#654](https://github.com/minodisk/riffle/issues/654)) ([c698c8e](https://github.com/minodisk/riffle/commit/c698c8e73d43009a3ad6b4d595d792ccbff9e40b))
* **claude:** keep existing settings.json entries during settings promotion ([#651](https://github.com/minodisk/riffle/issues/651)) ([cb63343](https://github.com/minodisk/riffle/commit/cb63343905abb4c6b119e11e32ad9e0aa6f37cfe))
* **core:** open big-endian (MM) DNGs ([#648](https://github.com/minodisk/riffle/issues/648)) ([b5f0397](https://github.com/minodisk/riffle/commit/b5f0397eda16f5ba6ffd9108d4795ca6938b52ac))
* **core:** return Err from the partial JPEG decode on a malformed JPEG ([#647](https://github.com/minodisk/riffle/issues/647)) ([425667a](https://github.com/minodisk/riffle/commit/425667a2a5d7a75d9ea043ede2fd9e5bcd15dbed))

## [1.2.0](https://github.com/minodisk/riffle/compare/v1.1.0...v1.2.0) (2026-10-01)


### Features

* **app:** add a View menu with the panel and view toggles ([#545](https://github.com/minodisk/riffle/issues/545)) ([e18c64d](https://github.com/minodisk/riffle/commit/e18c64dae9e759a41bc52cf1f765642c8842928b))
* **app:** add Expand All / Collapse All to the folder tree's menu ([#611](https://github.com/minodisk/riffle/issues/611)) ([f563e35](https://github.com/minodisk/riffle/commit/f563e351ae03736c00ec8a4b61227fd10dbdce45))
* **app:** add shadcn Neutral dark tokens and shared component classes ([#570](https://github.com/minodisk/riffle/issues/570)) ([e396513](https://github.com/minodisk/riffle/commit/e396513197ce247a8005e923685f1dcc66be2b50))
* **app:** ask for Lightroom's UI language in the first-launch dialog ([#612](https://github.com/minodisk/riffle/issues/612)) ([0ff57d1](https://github.com/minodisk/riffle/commit/0ff57d171a995ec33b1808b1b4e92198e91cb116))
* **app:** enlarge the settings modal and replace its tabs with a side navigation ([#580](https://github.com/minodisk/riffle/issues/580)) ([214879f](https://github.com/minodisk/riffle/commit/214879f53abd1870e01d0c9a2097a6835a50bb91))
* **app:** follow a folder rename in the trash runs and their undo entries ([#613](https://github.com/minodisk/riffle/issues/613)) ([7391b16](https://github.com/minodisk/riffle/commit/7391b16d2b44d6833c06c7fac4140662ef1220cf))
* **app:** move the folder tree pane onto the sidebar scheme ([#576](https://github.com/minodisk/riffle/issues/576)) ([f9202e2](https://github.com/minodisk/riffle/commit/f9202e282b5df7ef2df1ba5b2c7f99e467d3b098))
* **app:** move the format, trash and sequence dialogs onto the shared dialog classes ([#571](https://github.com/minodisk/riffle/issues/571)) ([109d846](https://github.com/minodisk/riffle/commit/109d8461b991a029cfcb482e4e65cabec7165115))
* **app:** move the menus, the strip bar and the filmstrip cells onto the shared tokens ([#574](https://github.com/minodisk/riffle/issues/574)) ([267ef14](https://github.com/minodisk/riffle/commit/267ef143ecbeec87d0a5c637634080b48a506d4f))
* **app:** move the sharpness score from the first scan pass to the second ([#624](https://github.com/minodisk/riffle/issues/624)) ([b142897](https://github.com/minodisk/riffle/commit/b142897dde78ed1ff4b103cb70f501d9a0cfb4ca))
* **app:** move the viewer overlays, the meta pane and the empty states onto the shared tokens ([#578](https://github.com/minodisk/riffle/issues/578)) ([b5696cc](https://github.com/minodisk/riffle/commit/b5696cc950f155de8fe5cc45e3e338e73c514cf1))
* **app:** re-extract HDR PQ CR3 files and list the EOS R8 ([#609](https://github.com/minodisk/riffle/issues/609)) ([da1788d](https://github.com/minodisk/riffle/commit/da1788dde7bb088466af32ad8ce16f812cf6a0ae))
* **app:** redo an undone Move Rejected to Trash with Edit &gt; Redo ([#551](https://github.com/minodisk/riffle/issues/551)) ([fc4ea63](https://github.com/minodisk/riffle/commit/fc4ea633c4567952d0cc59fea8cc77b23a8f1eab))
* **app:** rename the View menu's pane items to Folders and Metadata ([#602](https://github.com/minodisk/riffle/issues/602)) ([c0c85d2](https://github.com/minodisk/riffle/commit/c0c85d2df37bceb1b796330bb1ba47e3c5e31192))
* **app:** send the current file and the strip's visible range to the running scan ([#639](https://github.com/minodisk/riffle/issues/639)) ([3f8e9aa](https://github.com/minodisk/riffle/commit/3f8e9aa5555f6677666c2efc2f3cf7c9c5a1ecbf))
* **app:** show the sharpness score as the second scan pass fills it in ([#628](https://github.com/minodisk/riffle/issues/628)) ([7fab2bd](https://github.com/minodisk/riffle/commit/7fab2bd2c3ae2ef65df8956a2aeb75d060806f55))
* **app:** throttle the focus rescan and end an idle scan at once ([#608](https://github.com/minodisk/riffle/issues/608)) ([8847b54](https://github.com/minodisk/riffle/commit/8847b544ed0cfc9eaa7a27e9566a16eb5ffdb600))
* **app:** trim View to pane check items with modifier defaults ([#592](https://github.com/minodisk/riffle/issues/592)) ([81e6dcb](https://github.com/minodisk/riffle/commit/81e6dcb43147f7a0ba5570bae766826f600ec782))
* **core:** compute the eye focus cue and the sharpness score in one analysis pass ([#623](https://github.com/minodisk/riffle/issues/623)) ([e07d137](https://github.com/minodisk/riffle/commit/e07d137f121ed8afdc76f3f30bddb346a8e9677e))
* **core:** decode the HEVC PRVW / THMB of HDR PQ CR3 files to a JPEG ([#606](https://github.com/minodisk/riffle/issues/606)) ([88939f3](https://github.com/minodisk/riffle/commit/88939f3db9e430d27bae853af294be311103b9ad))
* **core:** give HDR PQ (HEIF) CR3 files a clear preview-not-supported message ([#595](https://github.com/minodisk/riffle/issues/595)) ([5c6a07b](https://github.com/minodisk/riffle/commit/5c6a07bfdb8069b9ddbe959c86a2560a5ae090f4))
* **core:** let the TIFF walker open an ORF header and a header-less MakerNote ([#588](https://github.com/minodisk/riffle/issues/588)) ([85de1ee](https://github.com/minodisk/riffle/commit/85de1ee0f50474f1efabdbcc12106d740c0f7cb0))
* **core:** read Canon CR3 files through the reader and the listing ([#566](https://github.com/minodisk/riffle/issues/566)) ([af6e119](https://github.com/minodisk/riffle/commit/af6e119e2262ae143020570a6b1c1c630d33078c))
* **core:** read Fujifilm RAF files ([#589](https://github.com/minodisk/riffle/issues/589)) ([ce8fee5](https://github.com/minodisk/riffle/commit/ce8fee57d6a260b88c0b99187f0617d0315ead84))
* **core:** read Nikon NEF files through the reader and the listing ([#564](https://github.com/minodisk/riffle/issues/564)) ([7911033](https://github.com/minodisk/riffle/commit/7911033a5ca2cc3b589fed591b49dc46ab321739))
* **core:** read OM System / Olympus ORF files ([#597](https://github.com/minodisk/riffle/issues/597)) ([024f6be](https://github.com/minodisk/riffle/commit/024f6bee123c33f76d7145fffaadfdf2d784ac5a))
* **core:** read the AF point from Canon and Nikon AFInfo2 (Step 5) ([#575](https://github.com/minodisk/riffle/issues/575)) ([815c535](https://github.com/minodisk/riffle/commit/815c535b7598acc853653d8e79afa6fcc8ad4763))
* **core:** read the Fujifilm RAF AF point from FocusPixel ([#607](https://github.com/minodisk/riffle/issues/607)) ([983849e](https://github.com/minodisk/riffle/commit/983849eb919d6cd9048933eb98b1066b8264244e))
* **core:** run the scan's workers below normal OS priority, the analysis pass lowest ([#631](https://github.com/minodisk/riffle/issues/631)) ([3e50855](https://github.com/minodisk/riffle/commit/3e508557dc3d907bf5e06dde59aad201d50c4ea2))
* **core:** take the scan's files from a shared queue the set_scan_focus command reorders ([#635](https://github.com/minodisk/riffle/issues/635)) ([1416d97](https://github.com/minodisk/riffle/commit/1416d972db1456d8f9c9b467bb570a78c524052d))
* **stats:** persist traffic and stars to the stats branch ([#538](https://github.com/minodisk/riffle/issues/538)) ([967c3c0](https://github.com/minodisk/riffle/commit/967c3c08d80f4837f417c852d5849aaf4801be58))


### Bug Fixes

* **app:** draw a folder that is itself a tree root only once ([#562](https://github.com/minodisk/riffle/issues/562)) ([7ec881b](https://github.com/minodisk/riffle/commit/7ec881bd60e3269d6a21128fddf6a55b07353f37))
* **app:** drop the doubled separator in the Windows and Linux File menu ([#573](https://github.com/minodisk/riffle/issues/573)) ([15d4e80](https://github.com/minodisk/riffle/commit/15d4e80c5a14c74d2549c17eea8918cb0139fb91))
* **app:** ignore case in tree.ts's relation, rebase and renameFolder on macOS and Windows ([#593](https://github.com/minodisk/riffle/issues/593)) ([1496fd6](https://github.com/minodisk/riffle/commit/1496fd6e6b00eadffb64bd06865fcce75bfe4d5f))
* **app:** keep focus on the deleted file's neighbour after a rescan ([#590](https://github.com/minodisk/riffle/issues/590)) ([74bd576](https://github.com/minodisk/riffle/commit/74bd5769bda9653d3948da7e5942a4541ff911e6))
* **app:** keep the rest of a shortcut override when one key conflicts ([#563](https://github.com/minodisk/riffle/issues/563)) ([1916328](https://github.com/minodisk/riffle/commit/19163281cce216ea7913d318204919e3d9d02ba4))
* **app:** land four small folder tree, viewer and payload fixes ([#558](https://github.com/minodisk/riffle/issues/558)) ([50150ee](https://github.com/minodisk/riffle/commit/50150ee8d1ce36a15a0a0c01fa08625640ca0ebe))
* **app:** pass the platform ignoreCase flag to the rename rebase and trash relation calls ([#630](https://github.com/minodisk/riffle/issues/630)) ([6d4d606](https://github.com/minodisk/riffle/commit/6d4d606d00ccae21fdc347cd9beef6dc7a9c0170))
* **app:** reveal a differently-cased open path in the folder tree ([#560](https://github.com/minodisk/riffle/issues/560)) ([8208a61](https://github.com/minodisk/riffle/commit/8208a6183d1b1a238178b6ae616b4df66f9b4f8b))
* **app:** wait for the Windows watch handle to close before renaming ([#567](https://github.com/minodisk/riffle/issues/567)) ([ec1e299](https://github.com/minodisk/riffle/commit/ec1e299d941cff78aa112d6e41cbef03baa94822))
* **core:** hand out orientation-neutral preview JPEGs ([#617](https://github.com/minodisk/riffle/issues/617)) ([0398325](https://github.com/minodisk/riffle/commit/039832561e85a2971567ead17352b0f8f029de6f))
* **core:** read the preview of the old OLYMP MakerNote in ORF files ([#618](https://github.com/minodisk/riffle/issues/618)) ([f50d20d](https://github.com/minodisk/riffle/commit/f50d20d2fd88edb89e7958cc1605cd600426a6cc))
* **core:** trim trailing spaces from the Exif Make and Model in the shared reader ([#629](https://github.com/minodisk/riffle/issues/629)) ([20144d1](https://github.com/minodisk/riffle/commit/20144d169427fc02760c67aa2cd4678e5b1da0f3))

## [1.1.0](https://github.com/minodisk/riffle/compare/v1.0.0...v1.1.0) (2026-09-28)


### Features

* **app:** add file rename from the strip's context menu ([#507](https://github.com/minodisk/riffle/issues/507)) ([101114d](https://github.com/minodisk/riffle/commit/101114d39cf09280de870553c24f1a294924d58f))
* **app:** add the rename_file backend command ([#505](https://github.com/minodisk/riffle/issues/505)) ([d1efbeb](https://github.com/minodisk/riffle/commit/d1efbeb44351cdac91a4847e81ff1ad3e53f9d0c))
* **app:** add trash-rejected right-click items to the folder tree ([#521](https://github.com/minodisk/riffle/issues/521)) ([8e3f4a9](https://github.com/minodisk/riffle/commit/8e3f4a950b18156eefd33458698e4a585226dd78))
* **app:** collect rejects over folders for trash_rejected ([#518](https://github.com/minodisk/riffle/issues/518)) ([2d80038](https://github.com/minodisk/riffle/commit/2d8003853fef9111da463a27e0dee7d516521760))
* **app:** confirm trashing rejects in a dialog with per-folder counts and the space freed ([#529](https://github.com/minodisk/riffle/issues/529)) ([52ea0d9](https://github.com/minodisk/riffle/commit/52ea0d9a9053a3213fa55570ef6db2489822501c))
* **app:** hold Clear Cache until the scan ends ([#522](https://github.com/minodisk/riffle/issues/522)) ([a253780](https://github.com/minodisk/riffle/commit/a2537809c8034fc5d3c83f5fb707f74708ec3675))
* **app:** hold Move Rejected to Trash and renames until the scan ends ([#519](https://github.com/minodisk/riffle/issues/519)) ([175b159](https://github.com/minodisk/riffle/commit/175b1594d0ac8e87376b842ece52f8197952d030))
* **app:** record each trash run and restore it on Windows and Linux ([#540](https://github.com/minodisk/riffle/issues/540)) ([7756f79](https://github.com/minodisk/riffle/commit/7756f792e411a216a79c907525e2cd4295b704d8))
* **app:** remove the folder-targeted items from the File menu ([#537](https://github.com/minodisk/riffle/issues/537)) ([d778edf](https://github.com/minodisk/riffle/commit/d778edf29818d67ad16de7bb84ce043c4d7a7655))
* **app:** replace the app icon with the film-frame design ([#532](https://github.com/minodisk/riffle/issues/532)) ([f0b3e8b](https://github.com/minodisk/riffle/commit/f0b3e8b01c6710b626ddf353126161473fdd0ba1))
* **app:** select several folders in the tree and trash their rejects together ([#526](https://github.com/minodisk/riffle/issues/526)) ([cf72d6f](https://github.com/minodisk/riffle/commit/cf72d6faff74549d86eb4cd7176431915e653141))
* **app:** show position/count on every burst member cell ([#510](https://github.com/minodisk/riffle/issues/510)) ([6c41ad0](https://github.com/minodisk/riffle/commit/6c41ad067c503832554bd5ecd6fb2f2643564e1c))
* **app:** sync the folder tree's watched set and re-list on tree-changed ([#520](https://github.com/minodisk/riffle/issues/520)) ([0a7c6d6](https://github.com/minodisk/riffle/commit/0a7c6d6b856261dc72cc0ebea1b19baff92cb151))
* **app:** undo a Move Rejected to Trash run with Edit &gt; Undo ([#549](https://github.com/minodisk/riffle/issues/549)) ([f91c7b8](https://github.com/minodisk/riffle/commit/f91c7b8b55b21bebc108a02a93271a72c84c7d8f))
* **app:** watch the folder tree's expanded folders from the backend ([#517](https://github.com/minodisk/riffle/issues/517)) ([b652608](https://github.com/minodisk/riffle/commit/b6526082f39ee7bca32476e6ab14bbb2dde3d33b))
* **skills:** add the delegate skill that hands a task to a new herdr worktree ([#511](https://github.com/minodisk/riffle/issues/511)) ([38a1e0d](https://github.com/minodisk/riffle/commit/38a1e0d580c4979c188de2802f98193406d06058))
* **stats:** snapshot release download counts daily to the stats branch ([#533](https://github.com/minodisk/riffle/issues/533)) ([243d578](https://github.com/minodisk/riffle/commit/243d5781b32abec25c87db8efa5a332d0a71565f))


### Bug Fixes

* **app:** keep the strip's scroll and thumbnails across a rescan ([#528](https://github.com/minodisk/riffle/issues/528)) ([05099f3](https://github.com/minodisk/riffle/commit/05099f351f49d67acc90b82ee2418d3189aeb211))
* **app:** keep the trashed URL on macOS and restore a run from it ([#547](https://github.com/minodisk/riffle/issues/547)) ([64b8bc8](https://github.com/minodisk/riffle/commit/64b8bc80aeebed52bbcbaca7e658c03f1f8d733d))
* **app:** move the selection with the focus on a single-file undo / redo ([#509](https://github.com/minodisk/riffle/issues/509)) ([3c0342e](https://github.com/minodisk/riffle/commit/3c0342e73b0e9a643b94d5ad6c4c59f158aff967))
* **app:** select only the resumed file when a folder reopens ([#535](https://github.com/minodisk/riffle/issues/535)) ([6e48e5d](https://github.com/minodisk/riffle/commit/6e48e5da9517ecd6bda8144a7d97c5ba89f5470c))
* **app:** show the Sequence folder-level error as a sticky error ([#512](https://github.com/minodisk/riffle/issues/512)) ([b77412c](https://github.com/minodisk/riffle/commit/b77412c80dcb8e178d4fea5b4b223060cd74d2d9))
* land five small todo fixes ([#543](https://github.com/minodisk/riffle/issues/543)) ([7aaab7a](https://github.com/minodisk/riffle/commit/7aaab7a67d8ed79397f87cfa83594af5c39ff175))

## [1.0.0](https://github.com/minodisk/riffle/compare/v0.4.0...v1.0.0) (2026-09-27)


### Features

* **app:** add sequence commands, run state and File menu item ([#465](https://github.com/minodisk/riffle/issues/465)) ([ce32bc5](https://github.com/minodisk/riffle/commit/ce32bc5638f9900c779d110a9d0c5731316fd0ef))
* **app:** add the rename_folder backend command ([#502](https://github.com/minodisk/riffle/issues/502)) ([9bb2d2f](https://github.com/minodisk/riffle/commit/9bb2d2ffcf7bb264d969a580652a90b50271dfe3))
* **app:** add the Sequence JPEG Timestamps frontend flow ([#466](https://github.com/minodisk/riffle/issues/466)) ([2c2c287](https://github.com/minodisk/riffle/commit/2c2c287ef75427e368dac51c2bdd505f73ab49ad))
* **app:** copy the folder path or name from the tree's context menu ([#487](https://github.com/minodisk/riffle/issues/487)) ([eb94c33](https://github.com/minodisk/riffle/commit/eb94c336b2c27f0d8c1d1a0182be23d5397165bd))
* **app:** divide the meta pane between groups, label the standard EXIF section, and rename Riffle to Analysis ([#444](https://github.com/minodisk/riffle/issues/444)) ([1292e4f](https://github.com/minodisk/riffle/commit/1292e4f7aa51813e4432b703a21b5ff07d11d0ef))
* **app:** expand, collapse and open folders with Right, Left and Enter in the tree ([#456](https://github.com/minodisk/riffle/issues/456)) ([f8fd6a4](https://github.com/minodisk/riffle/commit/f8fd6a40cd365ccaedd7563d3cf9fe0067da17af))
* **app:** flatten the meta pane into EXIF, Maker note and Analysis groups ([#448](https://github.com/minodisk/riffle/issues/448)) ([aae8278](https://github.com/minodisk/riffle/commit/aae827824c4ee6d29975989338ba3aada5e71e30))
* **app:** format the Sony AF fields and show them in the meta pane ([#440](https://github.com/minodisk/riffle/issues/440)) ([bc3358c](https://github.com/minodisk/riffle/commit/bc3358cf5b84990536390081612a681bbb228632))
* **app:** format the Sony drive, stabilization and picture settings and show them in the meta pane ([#442](https://github.com/minodisk/riffle/issues/442)) ([89e6b3a](https://github.com/minodisk/riffle/commit/89e6b3a7dfdf2e6a11c57d0c331f4ace3327f092))
* **app:** give the folder tree the keyboard with a cursor and gate the culling keys ([#454](https://github.com/minodisk/riffle/issues/454)) ([da474ea](https://github.com/minodisk/riffle/commit/da474ea23008e90ffe346732dc34d8542393e40b))
* **app:** highlight the changed part of the new time in the sequence preview ([#482](https://github.com/minodisk/riffle/issues/482)) ([488523d](https://github.com/minodisk/riffle/commit/488523d3912809be4e5a25c330da673604d2f7d5))
* **app:** left-align the strip bar tools and right-align the counter ([#438](https://github.com/minodisk/riffle/issues/438)) ([5532967](https://github.com/minodisk/riffle/commit/55329672016f26cf50a27d585c2df78a6918b929))
* **app:** list, scan and serve a JPEG-only folder and refuse culling on JPEGs ([#491](https://github.com/minodisk/riffle/issues/491)) ([169659f](https://github.com/minodisk/riffle/commit/169659f66b6aaef0bcbb5afe36e3a6311a926d8a))
* **app:** mark focus candidates on the strip and label the AF eye sharpness ([#450](https://github.com/minodisk/riffle/issues/450)) ([ebc9998](https://github.com/minodisk/riffle/commit/ebc99988a0f04647b2590db154f15bdb7275f772))
* **app:** offer a restart after a manual update ([#436](https://github.com/minodisk/riffle/issues/436)) ([8a3b7e9](https://github.com/minodisk/riffle/commit/8a3b7e97b0f241ad773ba58e3989ad14638c47e4))
* **app:** persist the eye sharpness in the index and remove the face-catch state ([#433](https://github.com/minodisk/riffle/issues/433)) ([5a9b340](https://github.com/minodisk/riffle/commit/5a9b340322d430a0eae3e310108a3e57b697797a))
* **app:** remove the Open folder button from the left pane ([#437](https://github.com/minodisk/riffle/issues/437)) ([6373961](https://github.com/minodisk/riffle/commit/63739616e622819be2569edf3cbccd1a07e3648e))
* **app:** rename a folder inline from the folder tree ([#504](https://github.com/minodisk/riffle/issues/504)) ([ecd5b31](https://github.com/minodisk/riffle/commit/ecd5b31f2af6cd0068c6ff6ae59c1b80bbd0672f))
* **app:** replace the Focus candidates filter item with an AF eye section ([#477](https://github.com/minodisk/riffle/issues/477)) ([90729bc](https://github.com/minodisk/riffle/commit/90729bcd6920a4d2c7e673c40968255b12b3304e))
* **app:** resume a folder at the last viewed photo ([#484](https://github.com/minodisk/riffle/issues/484)) ([f5306bd](https://github.com/minodisk/riffle/commit/f5306bdbd34cc9d6bd02eebe5e8b83d82a696b9a))
* **app:** reuse PhotoLab's registered Uuids in a fresh .dop ([#495](https://github.com/minodisk/riffle/issues/495)) ([9fb0bba](https://github.com/minodisk/riffle/commit/9fb0bba7cbd079f86da0e776f814c696e16eee87))
* **app:** reveal a folder in the OS file manager from the folder tree's right-click menu ([#467](https://github.com/minodisk/riffle/issues/467)) ([c8ab036](https://github.com/minodisk/riffle/commit/c8ab036bdce58c6d30ffdcadc4a083f10e6bcd7d))
* **app:** reveal the sequenced output folder after a run that wrote files ([#481](https://github.com/minodisk/riffle/issues/481)) ([674c7e3](https://github.com/minodisk/riffle/commit/674c7e38f7bfcde3be07234cfa32287e6adf6539))
* **app:** run the eye-sharpness pass after the scan and stream it to the frontend ([#439](https://github.com/minodisk/riffle/issues/439)) ([db13f7e](https://github.com/minodisk/riffle/commit/db13f7e3b220f4fb37ab2de9fbc3c9ac1812ba33))
* **app:** select every file in the strip with Cmd/Ctrl+A ([#462](https://github.com/minodisk/riffle/issues/462)) ([944cbc5](https://github.com/minodisk/riffle/commit/944cbc55520e9e630f1ba8257d2fcab1cb36e56a))
* **app:** sequence JPEG timestamps from the folder tree's right-click menu ([#475](https://github.com/minodisk/riffle/issues/475)) ([065bb23](https://github.com/minodisk/riffle/commit/065bb23067cf149f32550682e91fe29a559e213b))
* **app:** show meta rows and a filter for the focus candidate ([#441](https://github.com/minodisk/riffle/issues/441)) ([cbf26f9](https://github.com/minodisk/riffle/commit/cbf26f9249e3fea6e881f73a95d38833dc4a9dc0))
* **app:** store the AF eye in-focus probability as eye_focus and show it in the meta pane ([#476](https://github.com/minodisk/riffle/issues/476)) ([96772ba](https://github.com/minodisk/riffle/commit/96772baf89aa3ad7077380b35479971ed5c1a005))
* **app:** turn auto-advance on by default ([#496](https://github.com/minodisk/riffle/issues/496)) ([671b0b0](https://github.com/minodisk/riffle/commit/671b0b07ae1aaad29cbc2ca9acf9583c0feb932f))
* **app:** type-ahead jump to a folder by typing its name prefix ([#459](https://github.com/minodisk/riffle/issues/459)) ([800268b](https://github.com/minodisk/riffle/commit/800268b56c6f3b70c52aa6e4146abd46862f9c22))
* **app:** view-only mode for a JPEG-only folder in the strip ([#493](https://github.com/minodisk/riffle/issues/493)) ([85d4306](https://github.com/minodisk/riffle/commit/85d430679ca19e5c6fe871549644b0f9be03d756))
* **core:** compute the focus candidate cue from the eye sharpness ([#430](https://github.com/minodisk/riffle/issues/430)) ([fae80c2](https://github.com/minodisk/riffle/commit/fae80c2ae9060a63327159b43380a4ba2908af47))
* **core:** let dop::write_rating/write_label take a fresh sidecar's Uuids ([#490](https://github.com/minodisk/riffle/issues/490)) ([70d7b57](https://github.com/minodisk/riffle/commit/70d7b5703490b444880ad63085808f948f961ef9))
* **core:** port lapse's EXIF date-time patcher as sequence with time order and copy output ([#463](https://github.com/minodisk/riffle/issues/463)) ([16525e3](https://github.com/minodisk/riffle/commit/16525e32471929963978738d43400867c068eb95))
* **core:** read a JPEG's Exif and thumbnail through the existing entry points ([#488](https://github.com/minodisk/riffle/issues/488)) ([00a0a92](https://github.com/minodisk/riffle/commit/00a0a92b3f752edc7e9b6263776a1719d0dd94f4))
* **core:** read the Sony AF area, drive, stabilization and picture settings ([#434](https://github.com/minodisk/riffle/issues/434)) ([6d8a941](https://github.com/minodisk/riffle/commit/6d8a941e9da6469e151ad99b8bdaaa1e2f0bab22))
* **core:** score the AF eye by a combined in-focus probability ([#474](https://github.com/minodisk/riffle/issues/474)) ([256071d](https://github.com/minodisk/riffle/commit/256071d5cd4dfea77fd6c4549109f1fed16de772))


### Bug Fixes

* **app:** keep folder opens off the main thread and open on a row click ([#449](https://github.com/minodisk/riffle/issues/449)) ([e0e0114](https://github.com/minodisk/riffle/commit/e0e011417daadc8406ebf4dc4751ad95a2a820a8))
* **app:** use a thin scrollbar on the strip's menus ([#447](https://github.com/minodisk/riffle/issues/447)) ([509bfe8](https://github.com/minodisk/riffle/commit/509bfe8d290e433b68339923aa4ec2a53d5a0a2f))
* **core:** add the minimal Settings block PhotoLab 10 needs to .dop sidecars ([#470](https://github.com/minodisk/riffle/issues/470)) ([d233c6f](https://github.com/minodisk/riffle/commit/d233c6fe950ec55fbd971222953220a6548ef853))
* **core:** gate the manual-focus check on the model and close quick todo items ([#480](https://github.com/minodisk/riffle/issues/480)) ([2e49f0c](https://github.com/minodisk/riffle/commit/2e49f0c140ee7e609d61abf47f66360fb4f1547c))
* **core:** stop a canceled scan between each file's pipeline stages ([#451](https://github.com/minodisk/riffle/issues/451)) ([db3eec2](https://github.com/minodisk/riffle/commit/db3eec2434a5453e87a702a4ac194d5d4c386f02))
* **core:** write the RAW's EXIF Orientation into Riffle-made .dop sidecars ([#473](https://github.com/minodisk/riffle/issues/473)) ([7dfe474](https://github.com/minodisk/riffle/commit/7dfe47415315ceb446a4bd77d2c7f68eddda62bd))
* **mise:** match the lychee review-history exclusion on Windows paths ([#435](https://github.com/minodisk/riffle/issues/435)) ([42b040a](https://github.com/minodisk/riffle/commit/42b040a044c6b14ff914f710364b4389839351b2))


### Performance Improvements

* **app:** reuse list_arw's listing in scan_folder ([#455](https://github.com/minodisk/riffle/issues/455)) ([b662449](https://github.com/minodisk/riffle/commit/b662449f729935535a3dd929be022ba4ba8efccb))
* **app:** skip the scan-done refresh when neither the reconcile nor the scan changed a row ([#471](https://github.com/minodisk/riffle/issues/471)) ([89f4690](https://github.com/minodisk/riffle/commit/89f4690c78ed6e9c07ee41ecc345db05acf85d1e))
* **app:** time refreshEntries and drop its redundant runs ([#457](https://github.com/minodisk/riffle/issues/457)) ([9441d99](https://github.com/minodisk/riffle/commit/9441d993e044ca8e42b68c3c98225d378e9d1dc7))


### Documentation

* **readme:** simplify the compatibility lists ([#499](https://github.com/minodisk/riffle/issues/499)) ([935d697](https://github.com/minodisk/riffle/commit/935d697314dd697676fb3aa2f6858180e3705008))

## [0.4.0](https://github.com/minodisk/riffle/compare/v0.3.1...v0.4.0) (2026-09-25)


### Features

* **app:** add a Both sidecar format that writes the XMP and the .dop ([#406](https://github.com/minodisk/riffle/issues/406)) ([746b914](https://github.com/minodisk/riffle/commit/746b9143f7da73b0d31ae3ff115c913fed6bb818))
* **app:** add folder listing commands for the folder tree ([#410](https://github.com/minodisk/riffle/issues/410)) ([3fb3179](https://github.com/minodisk/riffle/commit/3fb31793422b085682c743f0608c912c36d80f03))
* **app:** add MCP get_photo and get_preview ([#417](https://github.com/minodisk/riffle/issues/417)) ([9761019](https://github.com/minodisk/riffle/commit/9761019c94c5c41cc5174b95c353c7143a4fde79))
* **app:** add the folder tree to the left pane ([#414](https://github.com/minodisk/riffle/issues/414)) ([2a0138c](https://github.com/minodisk/riffle/commit/2a0138c7706c0068e06804ae0105e7d80c9a549b))
* **app:** answer MCP get_view from the main window ([#409](https://github.com/minodisk/riffle/issues/409)) ([9074040](https://github.com/minodisk/riffle/commit/9074040a11bf09042894f4a61095f524fbf9fdba))
* **app:** color the focus mark by the face-catch state ([#404](https://github.com/minodisk/riffle/issues/404)) ([3c25f87](https://github.com/minodisk/riffle/commit/3c25f877f97d2a5fb589f75c586ebadeeb899cf7))
* **app:** draw the detected faces in the focus mark, detected on demand ([#411](https://github.com/minodisk/riffle/issues/411)) ([121bd7c](https://github.com/minodisk/riffle/commit/121bd7c73aae9b456da39af87c87ad85aab53980))
* **app:** drive the view from MCP tools ([#421](https://github.com/minodisk/riffle/issues/421)) ([13bd44d](https://github.com/minodisk/riffle/commit/13bd44d8b430dfe2513636ba2e80c812be7698d6))
* **app:** embed an MCP server behind a setting ([#402](https://github.com/minodisk/riffle/issues/402)) ([5cb4060](https://github.com/minodisk/riffle/commit/5cb40601acd9ce350b1139e6287b6e7c1fa02bc7))
* **app:** hide and show the left pane, filmstrip, and right pane from the keymap ([#416](https://github.com/minodisk/riffle/issues/416)) ([3775918](https://github.com/minodisk/riffle/commit/377591899a5a739e4305e5a5a200ce1328a9e792))
* **app:** move the filmstrip to the bottom and rotate the arrow keys ([#405](https://github.com/minodisk/riffle/issues/405)) ([4239333](https://github.com/minodisk/riffle/commit/4239333cdf3342310c9e4fc539896282c4c3c488))
* **app:** pick the Lightroom label preset from a language dropdown ([#424](https://github.com/minodisk/riffle/issues/424)) ([5d42d61](https://github.com/minodisk/riffle/commit/5d42d61988cdc97d6cc385bc8428c031171a28ea))
* **app:** replace the settings window with a modal in the main window ([#412](https://github.com/minodisk/riffle/issues/412)) ([fa57e8f](https://github.com/minodisk/riffle/commit/fa57e8f6eb0d547b7214f71dabc69d1fe2e1a572))
* **app:** write judgments from MCP through the UI's commit path ([#423](https://github.com/minodisk/riffle/issues/423)) ([9016e1e](https://github.com/minodisk/riffle/commit/9016e1e1a2ffa99e0f79cb61649bf30bbd00052f))
* **core:** load Lightroom color label presets from per-language JSON ([#422](https://github.com/minodisk/riffle/issues/422)) ([e366d0a](https://github.com/minodisk/riffle/commit/e366d0a5232009fa3f65367ba33f28ac276f3a10))


### Miscellaneous Chores

* wrap up lightroom-layout ([#420](https://github.com/minodisk/riffle/issues/420)) ([7c14bc6](https://github.com/minodisk/riffle/commit/7c14bc6042b60575df0d1d0672f5247b5f751232))

## [0.3.1](https://github.com/minodisk/riffle/compare/v0.3.0...v0.3.1) (2026-09-24)


### Features

* **app:** ask for the developing software on first launch ([#390](https://github.com/minodisk/riffle/issues/390)) ([66e3075](https://github.com/minodisk/riffle/commit/66e30754452e5e65947e139c13dc9ff0685cae21))
* **app:** draw the Sony AF frame in the focus mark ([#391](https://github.com/minodisk/riffle/issues/391)) ([933fd29](https://github.com/minodisk/riffle/commit/933fd298aa4704e72eca4e60c870f108a46436d0))
* **app:** group the meta pane into EXIF and Riffle ([#379](https://github.com/minodisk/riffle/issues/379)) ([1e812f7](https://github.com/minodisk/riffle/commit/1e812f793dc7f2c9e81760312f5bca4987a5c8cf))
* **app:** persist the face-catch state in the index ([#401](https://github.com/minodisk/riffle/issues/401)) ([1fc8c3f](https://github.com/minodisk/riffle/commit/1fc8c3f1c8f6e3e36112b637c1473b21cb4671ea))
* **app:** re-extract index rows written by an older extractor ([#382](https://github.com/minodisk/riffle/issues/382)) ([f4c77a8](https://github.com/minodisk/riffle/commit/f4c77a8e7de1179bb4edae13caf432458a6cca9d))
* **core:** compute the face-catch state through one detection entry point ([#397](https://github.com/minodisk/riffle/issues/397)) ([a2c8d9f](https://github.com/minodisk/riffle/commit/a2c8d9f3458d799d3dde0f9e60619f9211224121))
* **core:** read the Sigma BF AF point from the Sigma MakerNote ([#378](https://github.com/minodisk/riffle/issues/378)) ([e32f016](https://github.com/minodisk/riffle/commit/e32f01697fb2dd3bde26b35a67e8ce783dd20d83))


### Bug Fixes

* **app:** align the label-name inputs on a shared grid ([#389](https://github.com/minodisk/riffle/issues/389)) ([5123d62](https://github.com/minodisk/riffle/commit/5123d6224799fa4ab25db84fd7ba1834926847de))
* **app:** lower the Linux preview pixel limit to 6 MP ([#385](https://github.com/minodisk/riffle/issues/385)) ([61b1d20](https://github.com/minodisk/riffle/commit/61b1d2030009d4a1923e4711c7b9a8b2307f8695))
* **app:** remove the inherited menu bar from the settings window ([#388](https://github.com/minodisk/riffle/issues/388)) ([1e84e6a](https://github.com/minodisk/riffle/commit/1e84e6a47efaa489c57961fa4b5ec559f52675c6))
* **app:** resize oversized previews in the decode worker on Linux ([#383](https://github.com/minodisk/riffle/issues/383)) ([9988d10](https://github.com/minodisk/riffle/commit/9988d100a6d345d690ab8c4b944b70b9cae383a5))
* **core:** mask SHORT TIFF entries to 16 bits ([#376](https://github.com/minodisk/riffle/issues/376)) ([69cdda1](https://github.com/minodisk/riffle/commit/69cdda17339d1ed5e83d539e3e9d8b144aaf92a9))
* **core:** score the AF window when the AF point is off the face ([#396](https://github.com/minodisk/riffle/issues/396)) ([4bbdaaa](https://github.com/minodisk/riffle/commit/4bbdaaa44f50eaee8bb19daa78342fdcc12d260b))

## [0.3.0](https://github.com/minodisk/riffle/compare/v0.2.2...v0.3.0) (2026-09-23)


### ⚠ BREAKING CHANGES

* **app:** the `File > Open in DxO PhotoLab` menu item, the `photolab` shortcut action (Shift+Cmd+O / Ctrl+Shift+O) and the `open_in_photolab` Tauri command are removed.

### Features

* **app:** add rating and colour label items to the strip's context menu ([#363](https://github.com/minodisk/riffle/issues/363)) ([f4fb5e9](https://github.com/minodisk/riffle/commit/f4fb5e9fd65fc1a21270a138063829e3d161d9be))
* **app:** compare the sharpness cue within bursts ([#374](https://github.com/minodisk/riffle/issues/374)) ([38e9652](https://github.com/minodisk/riffle/commit/38e965287f51dec57459f1df651d65a226ce2da3))
* **app:** open the context menu on the main photo view ([#367](https://github.com/minodisk/riffle/issues/367)) ([d4e4d5a](https://github.com/minodisk/riffle/commit/d4e4d5adc796455f918c7286914255f7ae2ba54e))
* **app:** remove Open in DxO PhotoLab ([#375](https://github.com/minodisk/riffle/issues/375)) ([79556c7](https://github.com/minodisk/riffle/commit/79556c76240675e299de4445ec7babb8530e0778))
* **app:** show the app icon in the About dialog ([#368](https://github.com/minodisk/riffle/issues/368)) ([fe5b037](https://github.com/minodisk/riffle/commit/fe5b037f6a2422f681f03e35ae5558bf836b34b7))


### Bug Fixes

* **app:** fix strip cell burst band gap and selection outline ([#362](https://github.com/minodisk/riffle/issues/362)) ([4d71ddd](https://github.com/minodisk/riffle/commit/4d71dddde3bf43ba82757c04e776dae64eeb4fcd))
* **release:** publish the release only after the installers are attached ([#364](https://github.com/minodisk/riffle/issues/364)) ([bfcff56](https://github.com/minodisk/riffle/commit/bfcff56fa3d3737f5a59a193d0710f54c6409441))

## [0.2.2](https://github.com/minodisk/riffle/compare/v0.2.1...v0.2.2) (2026-09-22)


### Features

* **app:** add the labelNames setting through the app ([#356](https://github.com/minodisk/riffle/issues/356)) ([d9eb60c](https://github.com/minodisk/riffle/commit/d9eb60c3ba63dcebb85f229fc73eec1a77a32a9a))
* **app:** make undo and redo rebindable keymap actions ([#346](https://github.com/minodisk/riffle/issues/346)) ([e79c313](https://github.com/minodisk/riffle/commit/e79c313771896f42481a80fadf0e0c5366e71711))
* **app:** replace the app icon with the contact-sheet design ([#361](https://github.com/minodisk/riffle/issues/361)) ([8b40f20](https://github.com/minodisk/riffle/commit/8b40f200019ea6b1f326f046dcea250c637356d1))
* **app:** settings window for Lightroom label names ([#357](https://github.com/minodisk/riffle/issues/357)) ([6f15b9f](https://github.com/minodisk/riffle/commit/6f15b9f8dd4462e76a53418c05c5c9207630ddde))
* **app:** show the Sony shutter type in the meta pane ([#352](https://github.com/minodisk/riffle/issues/352)) ([6929e51](https://github.com/minodisk/riffle/commit/6929e510e09ac58255c6c1ed154476e59d2d839f))
* **core:** write and read configurable Lightroom label names ([#355](https://github.com/minodisk/riffle/issues/355)) ([fbea8b5](https://github.com/minodisk/riffle/commit/fbea8b5a839f7d2b0c580450551be1e3faab34fe))

## [0.2.1](https://github.com/minodisk/riffle/compare/v0.2.0...v0.2.1) (2026-09-22)


### Features

* **app:** add a momentary grayscale preview on a held key ([#338](https://github.com/minodisk/riffle/issues/338)) ([bf7f2f2](https://github.com/minodisk/riffle/commit/bf7f2f20685ddb3fd319e33900479729b064f162))
* **app:** add side-by-side comparison view ([#340](https://github.com/minodisk/riffle/issues/340)) ([1d4387d](https://github.com/minodisk/riffle/commit/1d4387d3698da6c95a59d6ef4cf26a07e6a6fe0e))
* **app:** carry the tri-state pick / reject flag through the app ([#343](https://github.com/minodisk/riffle/issues/343)) ([e442942](https://github.com/minodisk/riffle/commit/e442942817b8795b0c040b07a3377bbacd7ae73f))
* **core:** add a tri-state pick / reject flag with xmpDM:good ([#341](https://github.com/minodisk/riffle/issues/341)) ([acdd0fa](https://github.com/minodisk/riffle/commit/acdd0fa81597833d08186229165573be42a73470))
* **core:** read and write photoshop:LabelColor color labels ([#342](https://github.com/minodisk/riffle/issues/342)) ([dddb12b](https://github.com/minodisk/riffle/commit/dddb12bc0544c6bc6d14929f416ebcac93879c92))

## [0.2.0](https://github.com/minodisk/riffle/compare/v0.1.10...v0.2.0) (2026-09-22)


### Features

* **core:** parse Sony AFTracking and FocusFrameSize from the MakerNote ([#332](https://github.com/minodisk/riffle/issues/332)) ([e62fb18](https://github.com/minodisk/riffle/commit/e62fb1846bcec7d04cecd24faf283acf6030eeab))
* **core:** route the sharpness window through the Sony eye-AF frame ([#334](https://github.com/minodisk/riffle/issues/334)) ([0af87f3](https://github.com/minodisk/riffle/commit/0af87f382a073e569ff263f43a295d1ac1f0108c))


### Documentation

* **performance:** record Sony eye-AF window measurements ([#335](https://github.com/minodisk/riffle/issues/335)) ([fb3db85](https://github.com/minodisk/riffle/commit/fb3db85bd0a3a5f60bce4c6a53640ed2a30abaeb))

## [0.1.10](https://github.com/minodisk/riffle/compare/v0.1.9...v0.1.10) (2026-09-22)


### Features

* **app:** add a clearall action that clears every flag with one key ([#312](https://github.com/minodisk/riffle/issues/312)) ([c024382](https://github.com/minodisk/riffle/commit/c024382ec89a8397be64a9c597e0b457046953ea))
* **app:** add a pure strip selection model ([#307](https://github.com/minodisk/riffle/issues/307)) ([6d01931](https://github.com/minodisk/riffle/commit/6d01931993080099a7aadf2bc5e68b9bdb805ac6))
* **app:** add burst navigation keys ([#278](https://github.com/minodisk/riffle/issues/278)) ([174fa82](https://github.com/minodisk/riffle/commit/174fa821dd962ddbf2b5cc6b614af55133dc0ed5))
* **app:** add burstFramePrevious / burstFrameNext actions on Alt+ArrowUp/Down ([#320](https://github.com/minodisk/riffle/issues/320)) ([7d6bd99](https://github.com/minodisk/riffle/commit/7d6bd991c08cafe696bf6ec437055c2274f86cd8))
* **app:** add pure helpers for the strip flag menu ([#306](https://github.com/minodisk/riffle/issues/306)) ([4e54a1a](https://github.com/minodisk/riffle/commit/4e54a1a65f8e156f2b6b0cb4056db0dd80d9fc7b))
* **app:** apply judgments to every selected strip file ([#317](https://github.com/minodisk/riffle/issues/317)) ([7891bf7](https://github.com/minodisk/riffle/commit/7891bf71d1d2cc07448e8dfefbe4bb1a2be17a27))
* **app:** bump the index schema to v9 so sharpness scores are recomputed ([#272](https://github.com/minodisk/riffle/issues/272)) ([5ae9fd1](https://github.com/minodisk/riffle/commit/5ae9fd1c30610045a1ef607a9de75493b2aa53dc))
* **app:** group files into bursts and show them on the strip ([#276](https://github.com/minodisk/riffle/issues/276)) ([0f29a4e](https://github.com/minodisk/riffle/commit/0f29a4e5072a3ded7ac86049e9296f3a74c2b451))
* **app:** keyboard range extension and collapse for strip selection ([#323](https://github.com/minodisk/riffle/issues/323)) ([c9d7524](https://github.com/minodisk/riffle/commit/c9d752464f80b6ce7d5d54f4c824f5f5d2d431c9))
* **app:** pin the meta pane status lines to the bottom ([#299](https://github.com/minodisk/riffle/issues/299)) ([ce2b859](https://github.com/minodisk/riffle/commit/ce2b859fab0b1a12119ab1546f1e65c34cb53d8c))
* **app:** refresh the index cache size when a scan ends ([#315](https://github.com/minodisk/riffle/issues/315)) ([05c9a9b](https://github.com/minodisk/riffle/commit/05c9a9beb05430cfb8ecb92859d9f0de6e1478cb))
* **app:** reject the rest of the burst, undone as one entry ([#281](https://github.com/minodisk/riffle/issues/281)) ([98375ba](https://github.com/minodisk/riffle/commit/98375baf9082352b0c4a3441a312266f46ddc4d2))
* **app:** replace the burst bracket with a band and count badge ([#322](https://github.com/minodisk/riffle/issues/322)) ([a19f81d](https://github.com/minodisk/riffle/commit/a19f81d975a7af8914391f0f10ed5d47d43d102d))
* **app:** select multiple strip cells with Cmd/Ctrl+click and Shift+click ([#311](https://github.com/minodisk/riffle/issues/311)) ([f9d6442](https://github.com/minodisk/riffle/commit/f9d6442bce9ef26b673d94bc6ebafae4ea468801))
* **app:** wire the strip context menu into the main window ([#310](https://github.com/minodisk/riffle/issues/310)) ([6b78f0e](https://github.com/minodisk/riffle/commit/6b78f0eeae75a5d863d974126771e8a31dab8ab9))
* **core,app:** run face detection at scan time and bump schema to v10 ([#309](https://github.com/minodisk/riffle/issues/309)) ([5b1b684](https://github.com/minodisk/riffle/commit/5b1b68452b5f386720d329e1d12c67614011dece))
* **core:** add a face/eye detector prototype with a CLI benchmark ([#303](https://github.com/minodisk/riffle/issues/303)) ([3996e1b](https://github.com/minodisk/riffle/commit/3996e1b47a4d74ccc99f1f09aa9c0ec26b32bc91))
* **core:** parse the Sony FocusMode maker note tag into Shot ([#269](https://github.com/minodisk/riffle/issues/269)) ([2617703](https://github.com/minodisk/riffle/commit/26177030b8362c35c1fc2ac28795a0c9de1862d3))
* **core:** score sharpness on the eyes when a face is found ([#305](https://github.com/minodisk/riffle/issues/305)) ([c496d48](https://github.com/minodisk/riffle/commit/c496d4853f9f6325ad5b1ea9a8f6b046474610f3))
* **core:** score the sharpest tile when there is no trustworthy AF point ([#271](https://github.com/minodisk/riffle/issues/271)) ([73096bf](https://github.com/minodisk/riffle/commit/73096bfd733a94f74112b0f0e8d56eb7811e7211))


### Bug Fixes

* **app:** keep the strip scrollbar from clipping thumbnails ([#298](https://github.com/minodisk/riffle/issues/298)) ([68e978d](https://github.com/minodisk/riffle/commit/68e978d17591f106dfec42eb19809fa8921a6343))
* **app:** patch menu accelerators in place to keep the Windows menu bar dark ([#329](https://github.com/minodisk/riffle/issues/329)) ([8feb659](https://github.com/minodisk/riffle/commit/8feb6590adedefecc2caca1f7373a9f11469e274))
* **app:** scope the focus rescan to the main window ([#297](https://github.com/minodisk/riffle/issues/297)) ([ab28958](https://github.com/minodisk/riffle/commit/ab28958cda436b73c5500c4d48d10c86c8e00ee9))
* **merge:** rename the jq $label variable rejected by jq 1.6 ([#294](https://github.com/minodisk/riffle/issues/294)) ([c6f7140](https://github.com/minodisk/riffle/commit/c6f7140a1988bbc18f37c2cc357192e681e62443))
* **mise:** skip fast-forwarding main when any worktree has it checked out ([#330](https://github.com/minodisk/riffle/issues/330)) ([b1c6037](https://github.com/minodisk/riffle/commit/b1c6037195dd16a938025a5dcfa77a9c4cfe45cd))

## [0.1.9](https://github.com/minodisk/riffle/compare/v0.1.8...v0.1.9) (2026-09-21)


### Features

* **app:** add Edit &gt; Redo ([#261](https://github.com/minodisk/riffle/issues/261)) ([bafd49f](https://github.com/minodisk/riffle/commit/bafd49fb8ba0104a4737b309f79e09c91b986ec2))
* **app:** carry the flushed paths in scan-progress ([#254](https://github.com/minodisk/riffle/issues/254)) ([1b39ce5](https://github.com/minodisk/riffle/commit/1b39ce50b8c5d5db592b9b9f9afd9139cfe7b570))
* **app:** enlarge the dot and star on the app icon ([#255](https://github.com/minodisk/riffle/issues/255)) ([6e998ee](https://github.com/minodisk/riffle/commit/6e998ee0a3b267add9f5bca69e16bc61e0362aa9))
* **app:** forward timing lines to Riffle.log ([#265](https://github.com/minodisk/riffle/issues/265)) ([d8f0072](https://github.com/minodisk/riffle/commit/d8f007273aa4d7ecc4bf52a5351c54fa8fdb1eae))
* **app:** log a per-page timing line from the preview path ([#264](https://github.com/minodisk/riffle/issues/264)) ([7d49723](https://github.com/minodisk/riffle/commit/7d4972349bc156dca8d88dafa56107da401b1083))
* **app:** report the flushed paths from run_scan ([#253](https://github.com/minodisk/riffle/issues/253)) ([76b37a6](https://github.com/minodisk/riffle/commit/76b37a61071f543ca75a659e689056e01e519189))
* **app:** tint the macOS menu icons with the menu appearance ([#259](https://github.com/minodisk/riffle/issues/259)) ([ea2e1a8](https://github.com/minodisk/riffle/commit/ea2e1a834e5b2233385a1fe681187f389c8c6da2))


### Bug Fixes

* **app:** match the menu icon glyph size to the OS menu items ([#250](https://github.com/minodisk/riffle/issues/250)) ([29050f3](https://github.com/minodisk/riffle/commit/29050f36db9d9c75b5df2ab6a614801d7ef48c15))

## [0.1.8](https://github.com/minodisk/riffle/compare/v0.1.7...v0.1.8) (2026-09-20)


### Features

* **app:** add Index::clear to empty the folder index and reclaim the space ([#201](https://github.com/minodisk/riffle/issues/201)) ([ff28ab8](https://github.com/minodisk/riffle/commit/ff28ab89cc6986aca2668907851103c1d2e02953))
* **app:** add the index_size and clear_index commands ([#203](https://github.com/minodisk/riffle/issues/203)) ([91b2bce](https://github.com/minodisk/riffle/commit/91b2bce120e70f1d87cbf3e456d15251b2e2280b))
* **app:** add the Move Rejected to Trash menu item and frontend flow ([#240](https://github.com/minodisk/riffle/issues/240)) ([792aa1a](https://github.com/minodisk/riffle/commit/792aa1a6b3052647455ec6b5a1aa2ab0d5d2fdcf))
* **app:** add the settings window's Cache tab ([#204](https://github.com/minodisk/riffle/issues/204)) ([7137943](https://github.com/minodisk/riffle/commit/71379438d31a04366580992fce14bbafb6cdb960))
* **app:** add the trash_rejected command ([#237](https://github.com/minodisk/riffle/issues/237)) ([0ce13d9](https://github.com/minodisk/riffle/commit/0ce13d944dece437fa3b54f4f5172a406cccf4d6))
* **app:** add the viewer empty-state logic ([#217](https://github.com/minodisk/riffle/issues/217)) ([229896e](https://github.com/minodisk/riffle/commit/229896e807f77449dca1e89eb0a817048697e964))
* **app:** disable Clear Cache while a scan runs and show its progress ([#224](https://github.com/minodisk/riffle/issues/224)) ([c067a1a](https://github.com/minodisk/riffle/commit/c067a1a37a9b5e144cb5b3f0e827517b6f4f94ed))
* **app:** emit scan-state and index-clearing signals for settings ([#219](https://github.com/minodisk/riffle/issues/219)) ([b94c49c](https://github.com/minodisk/riffle/commit/b94c49c56bdba94d2ec2d9abdcfd586fd431aacf))
* **app:** filter the strip by portrait or landscape ([#239](https://github.com/minodisk/riffle/issues/239)) ([04b0d55](https://github.com/minodisk/riffle/commit/04b0d554a2fa5731084b0e334c2a98eb73acae98))
* **app:** rescan the open folder on focus and Reload Folder ([#216](https://github.com/minodisk/riffle/issues/216)) ([ecc50a1](https://github.com/minodisk/riffle/commit/ecc50a122d7b8ac6c2b766b1e66b5eef52a870f7))
* **app:** show a clickable empty state over the viewer ([#221](https://github.com/minodisk/riffle/issues/221)) ([26a9817](https://github.com/minodisk/riffle/commit/26a981775fa96d0611840352cf9e97823abfa109))
* **app:** watch the open folder and rescan on change (step 2) ([#220](https://github.com/minodisk/riffle/issues/220)) ([cc5168a](https://github.com/minodisk/riffle/commit/cc5168adb011b1ed36c0252406b4c78a46bf5323))


### Bug Fixes

* **app:** clear the scan entry when the scan task ends ([#212](https://github.com/minodisk/riffle/issues/212)) ([a39e8cc](https://github.com/minodisk/riffle/commit/a39e8cc48d27e19653e2abf3dd8c5322ea5654ae))
* **app:** drop modifier-only keys when the shortcuts setting is read ([#215](https://github.com/minodisk/riffle/issues/215)) ([21e418d](https://github.com/minodisk/riffle/commit/21e418d122ac34c70b1020a0dbe66889e7fbc282))
* **app:** hide a kept pick while XMP is the sidecar format ([#245](https://github.com/minodisk/riffle/issues/245)) ([a86c3e2](https://github.com/minodisk/riffle/commit/a86c3e2bad6207099f49f324972764eeac443d3d))
* **app:** keep the pick of a dirty row across a sidecar format switch ([#232](https://github.com/minodisk/riffle/issues/232)) ([491e5e6](https://github.com/minodisk/riffle/commit/491e5e6f799f55f2815659657f690fab218aa850))
* **app:** key sidecar read errors by the RAW path ([#211](https://github.com/minodisk/riffle/issues/211)) ([81907f3](https://github.com/minodisk/riffle/commit/81907f302b79733f803192d9c80ed5ea44fad6ac))
* **app:** make the filmstrip cell image box aspect-independent ([#207](https://github.com/minodisk/riffle/issues/207)) ([e95df70](https://github.com/minodisk/riffle/commit/e95df702a07f43f56045b52e89bdf60ecf21ae6c))
* **app:** open the filter menu as a fly-out beside the sidebar ([#238](https://github.com/minodisk/riffle/issues/238)) ([099b8fa](https://github.com/minodisk/riffle/commit/099b8fabd92faec06c9b165103ec288e9f8f115e))
* **app:** right-align the filter toggle in the tools row ([#249](https://github.com/minodisk/riffle/issues/249)) ([1194797](https://github.com/minodisk/riffle/commit/1194797cd6bcc091b9a968e024af8ca104ec2507))
* **core:** never rebind an xmp prefix bound to another namespace ([#213](https://github.com/minodisk/riffle/issues/213)) ([5210d6f](https://github.com/minodisk/riffle/commit/5210d6f71d9cc64d36b5da02099f28cfb04a0b66))

## [0.1.7](https://github.com/minodisk/riffle/compare/v0.1.6...v0.1.7) (2026-09-20)


### Features

* **app:** add File &gt; Open Folder… with keymap-driven accelerators ([#199](https://github.com/minodisk/riffle/issues/199)) ([e595692](https://github.com/minodisk/riffle/commit/e595692f608d989f654fda85113b7d0e28fb6021))
* **app:** add Help &gt; Open Log Folder to the app menu ([#194](https://github.com/minodisk/riffle/issues/194)) ([ed02749](https://github.com/minodisk/riffle/commit/ed02749fbcea6ab8713b1eaf5b149d6937ad6fdf))
* **app:** bundle SF Symbol icons for Settings and Undo menu items ([#193](https://github.com/minodisk/riffle/issues/193)) ([072b095](https://github.com/minodisk/riffle/commit/072b0959f596901198ccaab68313b0b2e100cf11))
* **app:** carry the full JPEG size in the focus_crop header for the zoom placeholder ([#173](https://github.com/minodisk/riffle/issues/173)) ([3a0cfbc](https://github.com/minodisk/riffle/commit/3a0cfbc86c1124b66be91b9a0e906fc7ccda9e34))
* **app:** evict stale folders from the index and VACUUM afterwards ([#176](https://github.com/minodisk/riffle/issues/176)) ([cd87f38](https://github.com/minodisk/riffle/commit/cd87f38c75da4ef15136654a9b79a7be8a4d4604))
* **app:** log the first scan's phases with their counts ([#189](https://github.com/minodisk/riffle/issues/189)) ([3baa581](https://github.com/minodisk/riffle/commit/3baa5812aa41317745b50562c8aa2dc320c5ba38))
* **app:** log the second-open path as open lines ([#192](https://github.com/minodisk/riffle/issues/192)) ([d4fff97](https://github.com/minodisk/riffle/commit/d4fff9743648839327c18c8ef2b30afb08f974fa))
* **app:** show native icons on the macOS menu items that have one ([#190](https://github.com/minodisk/riffle/issues/190)) ([c61c0a9](https://github.com/minodisk/riffle/commit/c61c0a995f25e3bb5a48e69239fa156c6214746c))
* **app:** trim the default shortcuts and unify the label defaults ([#191](https://github.com/minodisk/riffle/issues/191)) ([765ed06](https://github.com/minodisk/riffle/commit/765ed0676aed17e86ae5a0752004997553e80af9))


### Bug Fixes

* **app:** cancel the first-batch scan test deterministically ([#183](https://github.com/minodisk/riffle/issues/183)) ([76a6ea8](https://github.com/minodisk/riffle/commit/76a6ea861960c747cb8fa45029ffc9c9eb03ccc1))
* **app:** ignore lone modifier keys during shortcut capture ([#185](https://github.com/minodisk/riffle/issues/185)) ([46ff37a](https://github.com/minodisk/riffle/commit/46ff37a53c7cde5d0a5ac6ccd7cbec2f155a4e6c))
* **app:** keep shortcut overrides that are inactive under the current sidecar format ([#179](https://github.com/minodisk/riffle/issues/179)) ([69ee8fd](https://github.com/minodisk/riffle/commit/69ee8fd0b84e1a0420800ebf6915d2970f55a034))
* **app:** make the pick shortcut editable like any other action ([#180](https://github.com/minodisk/riffle/issues/180)) ([be8b3a3](https://github.com/minodisk/riffle/commit/be8b3a3c0412de0a268f57eca6adea93907f0133))
* **app:** retry a failed sidecar write with a bounded backoff ([#181](https://github.com/minodisk/riffle/issues/181)) ([1c23475](https://github.com/minodisk/riffle/commit/1c23475d1794c0d588099356e1ad2c1f89ef36b7))
* **app:** show sidecar problems in a sticky, dismissible error area ([#182](https://github.com/minodisk/riffle/issues/182)) ([e74a7ac](https://github.com/minodisk/riffle/commit/e74a7acc43faa11d318c81ba24192ab6756c1573))


### Performance Improvements

* **app:** list the folder once for both RAW files and sidecars in scan_folder ([#175](https://github.com/minodisk/riffle/issues/175)) ([9253c1c](https://github.com/minodisk/riffle/commit/9253c1c3b4564dd266e2ed1b03e4b8916bc6d881))

## [0.1.6](https://github.com/minodisk/riffle/compare/v0.1.5...v0.1.6) (2026-09-19)


### Features

* **app:** add a color label group to the filter menu ([#149](https://github.com/minodisk/riffle/issues/149)) ([931d42b](https://github.com/minodisk/riffle/commit/931d42be6882b5418ca4b9978c5da6817a4ae244))
* **app:** add a pure strip ordering module ([#141](https://github.com/minodisk/riffle/issues/141)) ([f3b5bcd](https://github.com/minodisk/riffle/commit/f3b5bcd2f5d36c05f3e24a4efe14f7e1cf413a01))
* **app:** add a sort choice to the strip pane ([#148](https://github.com/minodisk/riffle/issues/148)) ([7bee9db](https://github.com/minodisk/riffle/commit/7bee9db670305ccc2b098af354415d2a6f147bf3))
* **app:** add and remove individual shortcut keys per action ([#147](https://github.com/minodisk/riffle/issues/147)) ([50eea6b](https://github.com/minodisk/riffle/commit/50eea6b81052d17ca1dca4792c0cacc69b1e5306))
* **app:** add the Auto-advance toggle to the settings window ([#144](https://github.com/minodisk/riffle/issues/144)) ([5ef967d](https://github.com/minodisk/riffle/commit/5ef967d5ec10b2dc3ac34776a6c0c16c8bd1b4f2))
* **app:** add the autoAdvance setting to the backend ([#139](https://github.com/minodisk/riffle/issues/139)) ([062a0d9](https://github.com/minodisk/riffle/commit/062a0d98ab3425544bd9bdb1cb9666c105be5907))
* **app:** advance to the next file after a judgment when Auto-advance is on ([#150](https://github.com/minodisk/riffle/issues/150)) ([546dbb6](https://github.com/minodisk/riffle/commit/546dbb6ddf9f40cdb55a9db3f377d8b83eece982))
* **app:** bind shortcuts with any modifier combination ([#166](https://github.com/minodisk/riffle/issues/166)) ([70580ca](https://github.com/minodisk/riffle/commit/70580ca4d7df691f8b188365f40831fdf10c9109))
* **app:** install updates silently and add Check for Updates… ([#143](https://github.com/minodisk/riffle/issues/143)) ([d978438](https://github.com/minodisk/riffle/commit/d978438034ed4ffddc2d471fefa20669606da161))
* **app:** persist the strip sort order in the settings store ([#153](https://github.com/minodisk/riffle/issues/153)) ([5b047e3](https://github.com/minodisk/riffle/commit/5b047e3ee0495cb4427d392a83cdc40e412c1f38))
* **app:** read folder entries and thumbnails through a separate index connection ([#163](https://github.com/minodisk/riffle/issues/163)) ([7423537](https://github.com/minodisk/riffle/commit/74235373409a2caba63e3b3022dfb71338d17089))
* **app:** set the Riffle app icon ([#165](https://github.com/minodisk/riffle/issues/165)) ([9bbe44b](https://github.com/minodisk/riffle/commit/9bbe44bf90d5143923b5272c86a4ee86768c55a3))
* **app:** show each strip cell's sharpness relative to its neighbors ([#157](https://github.com/minodisk/riffle/issues/157)) ([b3b5fc7](https://github.com/minodisk/riffle/commit/b3b5fc7bffd9052897ae84ba663294f2827de2d9))
* **app:** show the open folder and file in the window title ([#123](https://github.com/minodisk/riffle/issues/123)) ([a09f12f](https://github.com/minodisk/riffle/commit/a09f12f653a1ff1d5426a7ef212aa78b080ba271))
* **app:** split the settings window into tabs ([#170](https://github.com/minodisk/riffle/issues/170)) ([b7a8af3](https://github.com/minodisk/riffle/commit/b7a8af3488f3cb5abe687e26e2d4fcc565f4840e))
* **app:** store the sharpness score in index schema v7 ([#152](https://github.com/minodisk/riffle/issues/152)) ([bace8de](https://github.com/minodisk/riffle/commit/bace8debdd0129749f0fed99598a934015aafb85))
* **app:** undo judgments with Edit &gt; Undo ([#136](https://github.com/minodisk/riffle/issues/136)) ([e463fd4](https://github.com/minodisk/riffle/commit/e463fd4b081c61c8d67fa4473bab0889cfc7464b))
* **core:** score preview sharpness around the focus point at scan time ([#145](https://github.com/minodisk/riffle/issues/145)) ([a2074df](https://github.com/minodisk/riffle/commit/a2074df8025773c87d2bf5e2d53d7ef762c47fea))


### Bug Fixes

* **app,core:** drop ping, release the crop bitmap and guard decode_rgb panics ([#140](https://github.com/minodisk/riffle/issues/140)) ([6767c18](https://github.com/minodisk/riffle/commit/6767c18d21c914af2e3e833e4f4d4d79ba7d390a))
* **app:** defer the Windows update install to quit ([#171](https://github.com/minodisk/riffle/issues/171)) ([c2bca4b](https://github.com/minodisk/riffle/commit/c2bca4bde58f3a20e673efa890c6710aff68aa44))
* **tools:** keep delete_merged_branches.sh from detaching the current worktree ([#167](https://github.com/minodisk/riffle/issues/167)) ([78903ad](https://github.com/minodisk/riffle/commit/78903adb49c52319276f2640d105334e92e9e6aa))
* **tools:** resolve origin/HEAD and fast-forward local main in git:main ([#146](https://github.com/minodisk/riffle/issues/146)) ([abbe752](https://github.com/minodisk/riffle/commit/abbe752ef8211b3ffa6e4628647211dea1650e95))

## [0.1.5](https://github.com/minodisk/riffle/compare/v0.1.4...v0.1.5) (2026-09-19)


### Features

* **app:** add per-format label shortcuts and Ctrl+Alt keys ([#116](https://github.com/minodisk/riffle/issues/116)) ([980d7a8](https://github.com/minodisk/riffle/commit/980d7a84443bb7379bc204a1ffb0e7b65a8e40df))
* **app:** carry the color label through the writer and set_rating ([#115](https://github.com/minodisk/riffle/issues/115)) ([1b95a4a](https://github.com/minodisk/riffle/commit/1b95a4a971da0b941311b63156fe5bd9925cd503))
* **app:** move settings into a separate settings window ([#120](https://github.com/minodisk/riffle/issues/120)) ([1662f0e](https://github.com/minodisk/riffle/commit/1662f0efe0c452783e1abc9c3f4a67baaa4687fa))
* **app:** show the color label and set it from the keymap ([#117](https://github.com/minodisk/riffle/issues/117)) ([8674546](https://github.com/minodisk/riffle/commit/8674546019165f115a8c74aa16f8920cc1ff8f4f))
* **app:** store the color label beside the rating in the index ([#114](https://github.com/minodisk/riffle/issues/114)) ([f3cfeea](https://github.com/minodisk/riffle/commit/f3cfeea65439efa34697ac0eb377d94cff49b22f))
* **core:** read and patch the .dop ColorLabel ([#113](https://github.com/minodisk/riffle/issues/113)) ([e9b77ec](https://github.com/minodisk/riffle/commit/e9b77ecbcf2c715a4ab8a18ae97490d73a6a319f))
* **core:** read and patch xmp:Label ([#111](https://github.com/minodisk/riffle/issues/111)) ([5834a22](https://github.com/minodisk/riffle/commit/5834a224271195e3b1368f745c41267560fecff3))

## [0.1.4](https://github.com/minodisk/riffle/compare/v0.1.3...v0.1.4) (2026-09-18)


### Features

* **app:** add EXIF groups to the filter menu ([#101](https://github.com/minodisk/riffle/issues/101)) ([30b8062](https://github.com/minodisk/riffle/commit/30b806213d96b052aa192a2abbbae0e2d68e3fbb))
* **app:** add shortcut rebind, reset and persistence commands ([#98](https://github.com/minodisk/riffle/issues/98)) ([0ece6f6](https://github.com/minodisk/riffle/commit/0ece6f68f2143906b4d8083fd7f071480eb1c6c1))
* **app:** add the Keyboard Shortcuts menu item and panel ([#102](https://github.com/minodisk/riffle/issues/102)) ([fb2d5aa](https://github.com/minodisk/riffle/commit/fb2d5aaf12defc9e6e1a94ec5c842f3a1592db0f))
* **app:** store the shooting settings in the index ([#100](https://github.com/minodisk/riffle/issues/100)) ([4d39c36](https://github.com/minodisk/riffle/commit/4d39c363b03e8e04629fe51ceb0b3ede17007df1))

## [0.1.3](https://github.com/minodisk/riffle/compare/v0.1.2...v0.1.3) (2026-09-18)


### Features

* **app:** dispatch culling keys through the resolved keymap ([#96](https://github.com/minodisk/riffle/issues/96)) ([9235a9f](https://github.com/minodisk/riffle/commit/9235a9ffee72b507cfce518f304cf416338b50f8))
* **app:** resolve the culling keymap from defaults and shortcut overrides ([#91](https://github.com/minodisk/riffle/issues/91)) ([c3fd55b](https://github.com/minodisk/riffle/commit/c3fd55b8f8a148c15cc31127ab714c96388b11f1))
* **app:** show update download progress and install errors ([#95](https://github.com/minodisk/riffle/issues/95)) ([8670401](https://github.com/minodisk/riffle/commit/867040145353d8b89c18327728b3182b0d1f2226))

## [0.1.2](https://github.com/minodisk/riffle/compare/v0.1.1...v0.1.2) (2026-09-18)


### Features

* **app:** add a pick and rating filter menu to the strip pane ([#88](https://github.com/minodisk/riffle/issues/88)) ([6d085e8](https://github.com/minodisk/riffle/commit/6d085e8c990241dc273d6f349d0152c042c5c58a))
* **app:** add the pick flag for .dop sidecars ([#84](https://github.com/minodisk/riffle/issues/84)) ([3307979](https://github.com/minodisk/riffle/commit/330797940447782bce9b0acd7066337122456c55))
* **app:** show pick/reject as a single dot in the strip cell ([#90](https://github.com/minodisk/riffle/issues/90)) ([6eaff32](https://github.com/minodisk/riffle/commit/6eaff321e63bdf8353e343b8d63942b0952b5cc3))

## [0.1.1](https://github.com/minodisk/riffle/compare/v0.1.0...v0.1.1) (2026-09-18)


### Features

* **app:** add single-instance, opener, and log plugins ([#75](https://github.com/minodisk/riffle/issues/75)) ([51c91ce](https://github.com/minodisk/riffle/commit/51c91ce6c15ef401ccc1a3d32120cb47a92dfccf))
* **app:** add the sidecar format setting (menu, switch, index reset) ([#83](https://github.com/minodisk/riffle/issues/83)) ([2e3ad90](https://github.com/minodisk/riffle/commit/2e3ad9027dcc32e43780397f4d41ed11e3c964ba))
* **app:** list DNG files next to ARW in app and CLI ([#73](https://github.com/minodisk/riffle/issues/73)) ([65cfc39](https://github.com/minodisk/riffle/commit/65cfc397ba05fd532dadaed6fd8dc4aea3bc7b35))
* **app:** open the folder in DxO PhotoLab from the menu ([#80](https://github.com/minodisk/riffle/issues/80)) ([87ebc4f](https://github.com/minodisk/riffle/commit/87ebc4f2f49a4910ede1b7f23f3f42f4dfbd817b))
* **app:** restore the window position and size across launches ([#74](https://github.com/minodisk/riffle/issues/74)) ([32b66c9](https://github.com/minodisk/riffle/commit/32b66c966252f70d3756c8ed435583e65c61b8ce))
* **app:** show an estimated aperture and the Leica focus distance ([#76](https://github.com/minodisk/riffle/issues/76)) ([0e92e29](https://github.com/minodisk/riffle/commit/0e92e296f35a2ec5557a3eaed95e579771c1b64c))
* **app:** write and reconcile the sidecar format selected in settings ([#78](https://github.com/minodisk/riffle/issues/78)) ([89bb2a4](https://github.com/minodisk/riffle/commit/89bb2a489d7ebe40eb3ed20c8568a8609229ecfc))
* **cli:** bench the 1:1 crop without FocusLocation and record DNG numbers ([#77](https://github.com/minodisk/riffle/issues/77)) ([95b313b](https://github.com/minodisk/riffle/commit/95b313b74da2a944064b70105ecf87f1b3bd5cc6))
* **core:** find the embedded JPEGs of a Leica DNG and gate the Sony MakerNote ([#69](https://github.com/minodisk/riffle/issues/69)) ([698fec9](https://github.com/minodisk/riffle/commit/698fec97622d83125e4fcaf41bc8ec733401be6d))
* **core:** read and patch DxO PhotoLab .dop sidecars ([#72](https://github.com/minodisk/riffle/issues/72)) ([57c50a2](https://github.com/minodisk/riffle/commit/57c50a2c5da13a8ebf476fc11df46a5afb282158))

## 0.1.0 (2026-09-18)


### Features

* **app-ui:** add ArrowUp/ArrowDown, WASD and HJKL paging keys ([#14](https://github.com/minodisk/riffle/issues/14)) ([b3787c8](https://github.com/minodisk/riffle/commit/b3787c8a9e5dd070571f418b4c0fb42390a69332))
* **app:** add a virtualized left thumbnail filmstrip ([#19](https://github.com/minodisk/riffle/issues/19)) ([2913a5e](https://github.com/minodisk/riffle/commit/2913a5e0773e7520549c3f7b6f4d52f959f9d22f))
* **app:** add an async focus_crop command returning a raw RGBA crop ([#30](https://github.com/minodisk/riffle/issues/30)) ([f839a21](https://github.com/minodisk/riffle/commit/f839a2158a1d47e5cc2f73e6b81a506d890b9255))
* **app:** add tauri-plugin-updater and an in-app update check ([#50](https://github.com/minodisk/riffle/issues/50)) ([2eadc47](https://github.com/minodisk/riffle/commit/2eadc47987dd8b9cca39bffaa8fa27777bbbf54a))
* **app:** draw the focus box on the preview ([#20](https://github.com/minodisk/riffle/issues/20)) ([808a58a](https://github.com/minodisk/riffle/commit/808a58afc37b4d8a608e44c870a7d8c235c557ce))
* **app:** draw the focus crosshair in green with a dark outline ([#48](https://github.com/minodisk/riffle/issues/48)) ([e450a3e](https://github.com/minodisk/riffle/commit/e450a3e14577b2e9b6cef9c56e9d79c8fdbf0c2d))
* **app:** frontend folder button, canvas, decode worker, arrow-key paging ([#7](https://github.com/minodisk/riffle/issues/7)) ([9d3adf2](https://github.com/minodisk/riffle/commit/9d3adf22dabe7edc22965632696f2204813f98ec))
* **app:** hide the focus box by default and draw it inverted ([#37](https://github.com/minodisk/riffle/issues/37)) ([39c4072](https://github.com/minodisk/riffle/commit/39c4072a22c0e52b5725c63129f2c7d02344e273))
* **app:** index folders in SQLite and scan them in the background ([#18](https://github.com/minodisk/riffle/issues/18)) ([0c5085f](https://github.com/minodisk/riffle/commit/0c5085f3ccfa807c5b4ecf501b1e7f61e29e8478))
* **app:** mark the focus point with a crosshair instead of a box ([#42](https://github.com/minodisk/riffle/issues/42)) ([4663d7c](https://github.com/minodisk/riffle/commit/4663d7c43ad1c84191424a5052fca57666d85b52))
* **app:** open a folder by drag-and-drop ([#21](https://github.com/minodisk/riffle/issues/21)) ([f610d71](https://github.com/minodisk/riffle/commit/f610d71c8610433f3e295786ad0973f489e3474b))
* **app:** rate files from the keyboard with rating badges ([#41](https://github.com/minodisk/riffle/issues/41)) ([6109790](https://github.com/minodisk/riffle/commit/6109790fa20d0104542c61b54b51f9de98b33f5d))
* **app:** reconcile sidecars on folder open, and flush dirty rows ([#36](https://github.com/minodisk/riffle/issues/36)) ([60941e2](https://github.com/minodisk/riffle/commit/60941e2fe95a11d17e807c0311dbb1de321ddfb3))
* **app:** reopen the last opened folder on launch ([#46](https://github.com/minodisk/riffle/issues/46)) ([b510a4b](https://github.com/minodisk/riffle/commit/b510a4bfa6ceb38c101ee6cf5d6e9618a3029ed1))
* **app:** report the read and decode time of a focus crop ([#51](https://github.com/minodisk/riffle/issues/51)) ([325be39](https://github.com/minodisk/riffle/commit/325be39fa57fe044ae6085d7c522f31076b9f504))
* **app:** Rust commands for folder pick, ARW listing, and preview bytes ([#6](https://github.com/minodisk/riffle/issues/6)) ([9e9deae](https://github.com/minodisk/riffle/commit/9e9deaec09c0fd0a5b8397bff5648f0fedbab045))
* **app:** scaffold Tauri 2 app crate and move CI to macOS ([#4](https://github.com/minodisk/riffle/issues/4)) ([1c43d3a](https://github.com/minodisk/riffle/commit/1c43d3ad327815581e85377c5843280c3540b6c6))
* **app:** show the shooting settings in a right metadata pane ([#26](https://github.com/minodisk/riffle/issues/26)) ([e6c33a0](https://github.com/minodisk/riffle/commit/e6c33a0eaba786d33ca0d64dd13dc6897d2e6e5d))
* **app:** store ratings in the index and coalesce XMP sidecar writes ([#33](https://github.com/minodisk/riffle/issues/33)) ([d721a00](https://github.com/minodisk/riffle/commit/d721a003e71f2d0f8b920143654f22bf019019ae))
* **app:** tidy up the rating display across canvas, strip, and meta pane ([#49](https://github.com/minodisk/riffle/issues/49)) ([cf1f3a5](https://github.com/minodisk/riffle/commit/cf1f3a55e7b9cc2f2b9dcc3a1a7664f6eacd992c))
* **app:** toggle a 1:1 focus check with Space ([#32](https://github.com/minodisk/riffle/issues/32)) ([a2decfc](https://github.com/minodisk/riffle/commit/a2decfc020af21f33fafadae4d7ee555997401a1))
* **app:** toggle timing logs from a Debug menu ([#44](https://github.com/minodisk/riffle/issues/44)) ([b821fa3](https://github.com/minodisk/riffle/commit/b821fa3c8bf5e883778abb9f1631d3230b990ea1))
* **core:** add a ranged JpgFromRaw read and focus-point RGBA crop ([#29](https://github.com/minodisk/riffle/issues/29)) ([c2eafea](https://github.com/minodisk/riffle/commit/c2eafeab5166be37c7a66c8537fdaa0b7439a566))
* **core:** parallel extraction with rayon and a CLI scan benchmark ([#17](https://github.com/minodisk/riffle/issues/17)) ([ff7f6f6](https://github.com/minodisk/riffle/commit/ff7f6f6a9ab1229a132e0c37f6a414d0ddd05d7e))
* **core:** read, write and patch XMP sidecar ratings ([#31](https://github.com/minodisk/riffle/issues/31)) ([c540dc7](https://github.com/minodisk/riffle/commit/c540dc7e2ed161c31f36912a18ad15211bcec9d9))


### Bug Fixes

* **app:** debounce the 1:1 crop request while the window resizes ([#60](https://github.com/minodisk/riffle/issues/60)) ([3f80bf0](https://github.com/minodisk/riffle/commit/3f80bf07f92e8bebee24246eb608fd2dc8ca86fd))
* **app:** keep the meta pane layout stable while paging ([#54](https://github.com/minodisk/riffle/issues/54)) ([f1ba56d](https://github.com/minodisk/riffle/commit/f1ba56d0dfee7833f2880f0f15bfefde6599e3b4))
* **app:** stop re-requesting every pending thumbnail on each scan tick ([#56](https://github.com/minodisk/riffle/issues/56)) ([d41b3bf](https://github.com/minodisk/riffle/commit/d41b3bf034738928a6e5e96ff7bd98ac213b1ec3))
* **app:** stop the folder dialog deadlocking the main thread ([#11](https://github.com/minodisk/riffle/issues/11)) ([e5d4fff](https://github.com/minodisk/riffle/commit/e5d4fff498f083bddf275f2ddeff07e2a08db599))
