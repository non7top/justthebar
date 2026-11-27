# Changelog

## [0.5.0](https://github.com/non7top/justthebar/compare/v0.4.0...v0.5.0) (2026-11-27)


### Features

* **bar:** a close button slides in from the screen edge on a hovered window button ([003735a](https://github.com/non7top/justthebar/commit/003735a85de9f14703d6fae4903f9a60bd058ce4))
* **bar:** a close button slides in from the screen edge on a hovered window button ([fd8cb87](https://github.com/non7top/justthebar/commit/fd8cb87ff54859402ac346732b7ba0427a9f2d64))

## [0.4.0](https://github.com/non7top/justthebar/compare/v0.3.0...v0.4.0) (2026-11-27)


### Features

* **mouse:** middle click on a window row launches a new instance ([aefac10](https://github.com/non7top/justthebar/commit/aefac10f2e924509dea3af53b826a6111560493c))
* **mouse:** middle click on a window row launches a new instance ([d7ef091](https://github.com/non7top/justthebar/commit/d7ef091f959aa5b51cf4a84b7ecc919e92b8e5d5))

## [0.3.0](https://github.com/non7top/justthebar/compare/v0.2.0...v0.3.0) (2026-11-24)


### Features

* **app:** --version flag, and version and provenance claim in the exe and hook DLL resources ([0342c1d](https://github.com/non7top/justthebar/commit/0342c1d90644549190331430141bcf51c42e7ad0))
* **installer:** per-user installer, --version and a signed release with Sigstore attestations ([0a3d9b6](https://github.com/non7top/justthebar/commit/0a3d9b6a542c9e58c020b648a0dd63756b3c2c05))
* **installer:** per-user NSIS installer that quits the running bar, replaces the files and offers a restart ([621228e](https://github.com/non7top/justthebar/commit/621228e8189fef2ec663bca72234e6d1f2e5980a))

## [0.2.0](https://github.com/non7top/justthebar/compare/v0.1.0...v0.2.0) (2026-11-24)


### Features

* **widgets:** a fat battery on its side with the number on it, glyphs centered on their ink ([5ede4f0](https://github.com/non7top/justthebar/commit/5ede4f0b2a3546661063a9ab3ba9caf8c2dc19fb))
* **widgets:** measured cells, stacked layout, drawn battery and volume icons, power cable state ([095ce86](https://github.com/non7top/justthebar/commit/095ce86b43f2d8b38b231e6282ced5605230ae76))
* **widgets:** scale the row down to a minimum before stacking, no labels on volume and battery, larger battery icon ([68a8007](https://github.com/non7top/justthebar/commit/68a80072863c405317348813ed2f9ed6b90d9673))
* **widgets:** show the input language as a two letter code ([5569806](https://github.com/non7top/justthebar/commit/556980610a9f2a4618a11226df5d6f0853e835d1))
* **widgets:** the battery is blue while the power cable is connected ([7b42e50](https://github.com/non7top/justthebar/commit/7b42e508409bc51796cb7d88996c6edc28acd555))


### Bug Fixes

* **startart:** handle the optional output buffer size of png 0.18 ([8fe7c21](https://github.com/non7top/justthebar/commit/8fe7c21480bcaefce699b308f583b81b1b0ea3d0))
* **startart:** read the PNG from a cursor, as png 0.18 needs a seekable reader ([3d959f3](https://github.com/non7top/justthebar/commit/3d959f38bde34b1fbe9a9168d195f5e306e68d65))
* **widgets:** draw the battery as tall as the signal glyph ([5299028](https://github.com/non7top/justthebar/commit/529902842e48d48de5d05e077bd3f0adfc0dcff7))
* **widgets:** fit the side bar widgets with scaled, measured cells, drawn battery and volume icons and no SSID polling ([cfd817d](https://github.com/non7top/justthebar/commit/cfd817ddf10104c4e62374069553d8f142f3cf50))
* **widgets:** read the Wi-Fi name only for the tooltip, add the volume bars and battery number, scale the row up and down ([7e6551a](https://github.com/non7top/justthebar/commit/7e6551aae3d3f4b5a1fd142cde9b2f90fa11f081))
* **widgets:** wrap bar widgets onto a second row on a side bar instead of shrinking them and cutting the labels ([e412830](https://github.com/non7top/justthebar/commit/e41283089d5e25a697b47a3fed06437706498739))

## 0.1.0 (2026-11-21)


### Features

* a lone Windows key press opens the Start menu at the bar's Start button ([410bf68](https://github.com/non7top/justthebar/commit/410bf68ee8bc2829f55c0f48a195626e67da1311))
* accordion headers for windows of one app, launch on click, fold on middle click ([162fbec](https://github.com/non7top/justthebar/commit/162fbec105af6ff42f0a9cc179797e5751cf7049))
* add month scroll buttons and anchor calendar to current month on top ([3e6631a](https://github.com/non7top/justthebar/commit/3e6631a70b7d6305fa4e73a7cbc7fbe6fef05ed1))
* add remote restart and quit commands ([ce5829a](https://github.com/non7top/justthebar/commit/ce5829a71aed48e23c026dd1da630607f73ea8d6))
* advanced options window built from a schema, opened by right click on the bar ([98dd094](https://github.com/non7top/justthebar/commit/98dd094ba00d97c527d9e15d43bdb372a7f114eb))
* advanced options window built from a schema, opened by right click on the bar ([3b4beff](https://github.com/non7top/justthebar/commit/3b4beff3c5ac0a80339942aeffafce14a1731ea9))
* answer ABM_GETTASKBARPOS with the bar whose Start button was clicked ([81dcb2e](https://github.com/non7top/justthebar/commit/81dcb2e38a88ae39a897361779dd157141c46e29))
* answer icon rectangle queries so apps place their menus at the icon ([a8ebd21](https://github.com/non7top/justthebar/commit/a8ebd215d86dac5143ec29e75b19ffaa1db1abac))
* AppBar taskbar engine with per-monitor task buttons ([906d811](https://github.com/non7top/justthebar/commit/906d811cfa145a68c2a9f2e46f19f357efa74650))
* AppBar taskbar engine with per-monitor task buttons, widgets, and flyout hook ([8aedb12](https://github.com/non7top/justthebar/commit/8aedb12321cf83e37060592925cd1040b600f257))
* calendar navigation buttons and top month anchoring ([2ae2c42](https://github.com/non7top/justthebar/commit/2ae2c42599c72f1591d1c552d5a6d7615f18620a))
* clicking the language widget shows the input language popup ([04ca1cc](https://github.com/non7top/justthebar/commit/04ca1cce4d0187ac445d0e018c77f12c4a00f713))
* clock formatting and native month calendar popup ([636f23f](https://github.com/non7top/justthebar/commit/636f23f6a4870981a3f794cc479984837b9799b7))
* config skeleton with appearance, tasks and reserved grouping sections ([6a31437](https://github.com/non7top/justthebar/commit/6a31437f9346f5448b0d478ef827f347b10f1798))
* configurable clock format with multi-line alignment ([6e475ba](https://github.com/non7top/justthebar/commit/6e475ba163e650df31d595e384c301feaa71a47b))
* configurable mouse actions for headers and windows ([b91c0f0](https://github.com/non7top/justthebar/commit/b91c0f033413cfde363519750e95a946ad76bd1c))
* decode notify icon packets ([6589109](https://github.com/non7top/justthebar/commit/65891095be4165d49cc4345937315da5c12f6232))
* dim the buttons of minimized windows ([df8e06b](https://github.com/non7top/justthebar/commit/df8e06b20f75e458db81a5778623523a5527aaeb))
* drag to reorder, drop to pin, right-click menu and persistent bar state ([e71bf78](https://github.com/non7top/justthebar/commit/e71bf78889a3617819c2a62dd91ca71ed82fb46c))
* drag windows within a group, log drags ([76c00bf](https://github.com/non7top/justthebar/commit/76c00bf14e9d684cd9ea60e6c4eddb20c8fe4725))
* draw tray icons on the tray bar ([13a0ab5](https://github.com/non7top/justthebar/commit/13a0ab5c0372d05f1be5d3f563af9890de623229))
* drop tray icons of exited apps and follow an Explorer restart ([1b6fddd](https://github.com/non7top/justthebar/commit/1b6fddd70fe0bb6890774bc7b4d32d9e913c6b13))
* forward tray icon clicks and hover to the owning app ([90f104f](https://github.com/non7top/justthebar/commit/90f104fd8c3de2401b288e06a43af982c4780681))
* header click folds, middle click launches ([ba60333](https://github.com/non7top/justthebar/commit/ba60333f84adfae108e83127382b57ed354c426e))
* highlight the task button under the mouse ([f8b658b](https://github.com/non7top/justthebar/commit/f8b658b0bc19496623eb609fb9b11756922175fa))
* language click switches the layout directly, larger widget glyphs ([8821cb4](https://github.com/non7top/justthebar/commit/8821cb4d1bce166adbaeebdbff6308a45f193ebc))
* language widget opens a menu of the installed keyboard layouts ([d2bc8ca](https://github.com/non7top/justthebar/commit/d2bc8cabf7ef0702c16ea8c559b03a9c3689c593))
* leave Explorer's quick settings icons (power, network, volume) out of the tray ([214f4fd](https://github.com/non7top/justthebar/commit/214f4fd475adcfc0cff33b280d95b70e965f1db9))
* log the flyout's visual frame bounds after moving it ([e5183e9](https://github.com/non7top/justthebar/commit/e5183e94648e4cd2b7778276b426e11094ddfc52))
* mark buttons whose window title changed with an orange stripe ([d622a8e](https://github.com/non7top/justthebar/commit/d622a8e3db59d8c651844fb9aa490358013c7dfc))
* mark windows that flash for attention ([263799d](https://github.com/non7top/justthebar/commit/263799d0849ebacc2e1a23eee93b3cff370adab7))
* model accordion groups of windows by executable ([3fd722b](https://github.com/non7top/justthebar/commit/3fd722b25d1c21e24ea8032c9f076e5ec99fbc4a))
* move Open-Shell's button under the Start button before toggling the menu ([4a7d8c2](https://github.com/non7top/justthebar/commit/4a7d8c2fb45ead32138457d743c0c8c37ba8ca2c))
* move the quick settings flyout next to the bar whose widget was clicked ([6943091](https://github.com/non7top/justthebar/commit/69430916c090ee8c8fc9119760e4a0cc76af29e0))
* multi-month stacked calendar view ([cab2b13](https://github.com/non7top/justthebar/commit/cab2b1306808bcc5b641219cad6354246e016af0))
* open the Start menu in process by default ([1919d02](https://github.com/non7top/justthebar/commit/1919d0272430303bb7de825f550432835c39f28c))
* optional in-process Open-Shell menu with explicit placement (start.in_process) ([369f3dc](https://github.com/non7top/justthebar/commit/369f3dc5da84c499a3054e0c4ccab1564ab71aa0))
* place the quick settings flyout from inside ShellHost with a small hook DLL ([fe782e2](https://github.com/non7top/justthebar/commit/fe782e20de8d85951e69ac9338d9a1d81ea0d443))
* popup month calendar control on clock click ([1c7cec4](https://github.com/non7top/justthebar/commit/1c7cec4d18a9405431af4de5c48bd3a628ae3c9d))
* post Open-Shell's toggle message to Explorer instead of starting StartMenu.exe ([acdf36f](https://github.com/non7top/justthebar/commit/acdf36f5a47a8a92827853fc5e2b4734e81952e5))
* read Start button art in Open-Shell's strip format ([09b6009](https://github.com/non7top/justthebar/commit/09b600966eb567628647b9a6533d3bb5cd769d86))
* right click on the Start button shows Open-Shell's menu, Shift+right click the Win+X menu ([c84fdca](https://github.com/non7top/justthebar/commit/c84fdca2279f9982ac93307cb5d4149f8a6ff211))
* shell helpers for file drops, shortcuts and popup menus ([7d1c2e9](https://github.com/non7top/justthebar/commit/7d1c2e92244ac9feee290f606674193716ce90ee))
* show the app icon on pin headers ([2a3b961](https://github.com/non7top/justthebar/commit/2a3b961425ad92177db7156163d70155d489c80f))
* show the dragged button and where it will land ([e4f8f33](https://github.com/non7top/justthebar/commit/e4f8f33b78c5aea5bcd2fc5c7d604eebfb78b578))
* start button section in the config ([d930b84](https://github.com/non7top/justthebar/commit/d930b84ac5e602ea89c9d2a624dc8136345f9245))
* Start button with art, a mix fade and a drawn fallback ([6501009](https://github.com/non7top/justthebar/commit/6501009d9221d63e7bec27def37a776cc64ac1d4))
* start.menu auto uses Open-Shell when installed, else the built-in menu ([628a9e0](https://github.com/non7top/justthebar/commit/628a9e08ec98768a5fbe7cd370a98759baf138e4))
* start.size sets the Start orb size ([8a4b771](https://github.com/non7top/justthebar/commit/8a4b771023825e8ea982a20a99ac8a72e4088d32))
* state model for fold, order and pins added from the bar ([8626872](https://github.com/non7top/justthebar/commit/86268721a4365ebfccaaf5025f286c8e7ccd2a52))
* stock flat two-pane start menu widget ([e8c775c](https://github.com/non7top/justthebar/commit/e8c775c15565de4fb53e2a05021c92c233f16a7a))
* stock flat two-pane start menu widget ([4c0447b](https://github.com/non7top/justthebar/commit/4c0447b9c0369fcff96bd793755eee910e2311cb))
* stretch Explorer's hidden taskbar window to our bar when the Start menu opens ([7bd87eb](https://github.com/non7top/justthebar/commit/7bd87ebbb4d6d562aec96cc3a9fa995e04c66b79))
* support multi-month stacked calendar view ([9064a0f](https://github.com/non7top/justthebar/commit/9064a0f187733f89d1e9047385d01b6b798be96d))
* tint the battery glyph green while charging and red when low ([8c31132](https://github.com/non7top/justthebar/commit/8c311327718da526c57f3ae16226b0fe4d11ef3b))
* tooltip for tray icons ([58a1896](https://github.com/non7top/justthebar/commit/58a18969086d336598a1e092e956d406282ae350))
* tray icon model in core ([b883882](https://github.com/non7top/justthebar/commit/b8838825fe327f780ce19158ea26c8afe5daeaa3))
* tray section in the config ([c2bf282](https://github.com/non7top/justthebar/commit/c2bf28243c7394fee1596be8a4fbd8e633ce2ba0))
* tray window that logs notify icons and forwards the rest to Explorer ([0957192](https://github.com/non7top/justthebar/commit/0957192100d209014f3537c548df4466c3702abd))
* tray.hide drops the icons of listed executables ([1773740](https://github.com/non7top/justthebar/commit/17737407dcde8a2f82eef29b256bc8d385e06d4f))
* unpin config pins by hiding them in the state file ([cd44a51](https://github.com/non7top/justthebar/commit/cd44a51cfc037207acede51d0ac9889fce46461c))
* validate custom groups and pins when the config loads ([adb0448](https://github.com/non7top/justthebar/commit/adb0448f39babe84558e9ce5a397f221bc6e2582))
* volume and network widgets, battery opens quick settings ([3cab7ad](https://github.com/non7top/justthebar/commit/3cab7ad9af1dc458cb88a5a2881a0ad43811c6d9))
* widget framework with battery and language widgets ([4491384](https://github.com/non7top/justthebar/commit/44913840b7fc0d329db74eb030c8bcd133e37a48))
* widgets on the bar itself or in the tray, three letter language code ([fb050e1](https://github.com/non7top/justthebar/commit/fb050e1b28a4bcb4733353d9912401e1823c6966))
* window activity lifecycle and notification rules for coding agents ([28ba867](https://github.com/non7top/justthebar/commit/28ba8673383ee739520357c2e152839bc58841ac))
* window activity lifecycle and notification rules for coding agents ([bde5960](https://github.com/non7top/justthebar/commit/bde5960c9b1d2f773d8d033817c3e46ce0bf7d70))


### Bug Fixes

* a click on the bar no longer takes the foreground, so the active window's button minimizes ([26e81a2](https://github.com/non7top/justthebar/commit/26e81a2be766f626e42e5c4de25f2fb5eb0bc295))
* a click that dismisses the flyout does not reopen it ([a18160b](https://github.com/non7top/justthebar/commit/a18160b107f01b04595b718de4fd9faa21eea7f0))
* cap the Start orb at 54 px and center it on thick bars ([caee8dd](https://github.com/non7top/justthebar/commit/caee8ddf2b75eae15964b43cb18307fb682b4615))
* do not hold the widget list borrowed while a click opens a URI ([0958c07](https://github.com/non7top/justthebar/commit/0958c0746e4de92b3bfedcf5a3aafe6ff7d66547))
* do not hold the Windows key after switching the input language ([2995312](https://github.com/non7top/justthebar/commit/29953129dca40b3136709001a1c79ece633fe77d))
* do not move the flyout after the hook placed it, keep y on side bars, use the bar's dpi ([442ab47](https://github.com/non7top/justthebar/commit/442ab47e0935967dd614d436575fd4ced33c615f))
* do not send Win+A while the flyout is open, log whether the click closed it ([b103072](https://github.com/non7top/justthebar/commit/b1030722966a96e98dfab17c1579a919330cbf69))
* embed a supportedOS manifest so Open-Shell's library sees Windows 10 ([0b8f916](https://github.com/non7top/justthebar/commit/0b8f9162678b06ff8413c40d5c55ed32fcfa5802))
* **fullscreen:** leave the bars alone while a fullscreen window is up ([32c7d2e](https://github.com/non7top/justthebar/commit/32c7d2e3b49d8b08166c85e119c58486ff45f479))
* give every hook DLL build its own message and subclass so a new build replaces a loaded one ([daa6712](https://github.com/non7top/justthebar/commit/daa6712ffa6c598577724e066a9fc9e058d369dd))
* give the in-process menu its own comctl32 v6 context and close its managers on exit ([439d88e](https://github.com/non7top/justthebar/commit/439d88ed402898422aff28e66fa179c73bea3d2c))
* hide minimized windows into the bar at startup and after an Explorer restart ([26778a5](https://github.com/non7top/justthebar/commit/26778a5a7e06e4eecbcf8cba7dfe372a6fceedb3))
* hold shell changes for 6 s after a fullscreen window and log what it was ([05a7e03](https://github.com/non7top/justthebar/commit/05a7e03fcaad5dbea22619db18daadc3891a4674))
* keep Open-Shell's button at the Start button until the menu has opened ([20a5156](https://github.com/non7top/justthebar/commit/20a515673fa3b7c2c90c6b6396cbaea75386c4ca))
* keep the bars behind a fullscreen window on their monitor ([5ec5b75](https://github.com/non7top/justthebar/commit/5ec5b7566433f50dcb40bca05b190d3c2d3dff44))
* lay the bars out once after a burst of display changes, not while a fullscreen window is up ([c2d9fc3](https://github.com/non7top/justthebar/commit/c2d9fc3b6f900b7194bf1ae1dedcfaae48ccb5ec))
* **layout:** keep the bars when a display change leaves the monitors the same ([4118446](https://github.com/non7top/justthebar/commit/4118446c49d6189ef6956a369a3cbcc470b21cd5))
* let Explorer take the foreground for the direct toggle, log Open-Shell's geometry per click ([ac57a59](https://github.com/non7top/justthebar/commit/ac57a598ec46786265ccd9a514d0499c592c5423))
* load Shcore.dll so Open-Shell reads the clicked monitor's DPI ([d48071c](https://github.com/non7top/justthebar/commit/d48071ca327773db360fa438d28fac3926b0fd2b))
* log the foreground window on fullscreen changes ([3593c23](https://github.com/non7top/justthebar/commit/3593c23b9013fb98e93e404d608f05daa4a416eb))
* log the language switch and also ask the focused control ([6e75e8f](https://github.com/non7top/justthebar/commit/6e75e8fbed128d90b42ec3147bdf6e4efad61090))
* measure the clicked monitor's DPI for the in-process menu (built, not tested) ([7138e70](https://github.com/non7top/justthebar/commit/7138e702a0e95ab520f1efa08de37a433faf82ab))
* minimize and restore task windows with system commands and log the result ([a092288](https://github.com/non7top/justthebar/commit/a092288edb43338bcd0cfd1f0ba293e32fbb5dc1))
* move the parked flyout window once before the first open so the hook can place it ([d357f3c](https://github.com/non7top/justthebar/commit/d357f3cfafb969fd1d3749a9249571cf8043c937))
* open quick settings with Win+A instead of an unconfirmed URI ([900bed9](https://github.com/non7top/justthebar/commit/900bed9cb36c071f8cee7124c802f845c1164fcb))
* open the Win+X menu by posting its hotkey to Explorer's taskbar ([65b5ad4](https://github.com/non7top/justthebar/commit/65b5ad4e6eebbce1309becb64b6636c4893d4d13))
* Open-Shell is the default start menu; language menu reopen, stock menu DPI, first flyout position ([69438c9](https://github.com/non7top/justthebar/commit/69438c9c5a1199a9c557cdf653b463b86967e8fe))
* place bars flush at startup unless Explorer was already auto-hiding before we hid it ([df0a038](https://github.com/non7top/justthebar/commit/df0a0382423c73d3cc86a35713f0ffabc50e95b8))
* place the flyout from its size at the show, a new window has none ([de73428](https://github.com/non7top/justthebar/commit/de734281d0bd04d1bd3f0dcd4d12335776ba878c))
* pre-move the flyout also when the shell hides it by cloaking ([d5c0444](https://github.com/non7top/justthebar/commit/d5c0444863275ea69d5fc8e6ce181a0d23877eba))
* rebuild the appbar registration when re-sending the position does not reserve the space ([0bdc3fd](https://github.com/non7top/justthebar/commit/0bdc3fd3106c395a61f3eae0530f4c0a1d18e9c1))
* refuse a tray modify for an unknown icon instead of adding it; drop tray.hide ([8c32ada](https://github.com/non7top/justthebar/commit/8c32ada3076cad101c0304bb308daa382085bfae))
* refuse a tray modify only for an icon its app deleted ([fb4b0a5](https://github.com/non7top/justthebar/commit/fb4b0a5e0dacfbc89927ec20ac34c5bb5cb6b645))
* reserve the bars' space again when Windows resets the work area ([f6a0914](https://github.com/non7top/justthebar/commit/f6a0914f97850c6a74eaca51ba17ad91f4c3ea8b))
* run Open-Shell's menu on a thread of its own so the bar does not freeze ([e39512e](https://github.com/non7top/justthebar/commit/e39512e2b6153605adf7e69cd0d43d1608c74e2a))
* satisfy clippy in the Open-Shell log reader ([5e4306a](https://github.com/non7top/justthebar/commit/5e4306ab4461faa8a505cc583e418fdd6b173890))
* set the work area directly when Explorer ignores the appbar messages ([960204c](https://github.com/non7top/justthebar/commit/960204cd3c0606102bd81dbf1ef9092dded0c2e6))
* shrink icons with an area average before upload, so the large Start art is not aliased ([65bce19](https://github.com/non7top/justthebar/commit/65bce1982099d50911bc085977c7b386ef0d2b0e))
* size tray tooltips without TTM_GETBUBBLESIZE, which crashed comctl32 ([65b42f6](https://github.com/non7top/justthebar/commit/65b42f6ef3e5387352dc0b19eae72bcba217f334))
* **start:** do not reopen Open-Shell's menu that the Start press closed ([8b4e475](https://github.com/non7top/justthebar/commit/8b4e4756012f51e9282bf69a46e46f0363b4d4bc))
* stop Explorer-side Start buttons taking focus from fullscreen games, leave the taskbar alone during them ([d2bb1fb](https://github.com/non7top/justthebar/commit/d2bb1fbbd08e024ebf81658789d97b4c3c3e8e31))
* stop moving the quick settings flyout once a click is closing it ([155ee64](https://github.com/non7top/justthebar/commit/155ee641cd4ca030e0f49f96e42f874626ca52c9))
* stop setting the work area directly, refuse modifies of unknown tray icons, log what Explorer holds for a bar ([69b5231](https://github.com/non7top/justthebar/commit/69b5231057b1c2e73a6453aaea444b18b05600bc))
* the Start menu Explorer entry opens This PC like Open-Shell's default ([6900d4a](https://github.com/non7top/justthebar/commit/6900d4a188297442ff18b3f1d434543751003335))
* **tooltip:** let the pointer pass through the tray tooltip so it stops blinking ([8be07dc](https://github.com/non7top/justthebar/commit/8be07dc7c1592b81175b9d9df561b93d6860f244))
* **tray:** drop the microphone icon when no app is using the microphone ([070dbbc](https://github.com/non7top/justthebar/commit/070dbbc354171618fab999a7e05ea22ff4d48f76))
* **tray:** refuse modifies of an icon the same program deleted, even from a new instance ([6e2118c](https://github.com/non7top/justthebar/commit/6e2118ca7f486aa3e9aeda48e9f96da8546766c4))
* widgets in one row on side bars, quick language refresh, volume COM init, adapter fallback for network ([8f41b64](https://github.com/non7top/justthebar/commit/8f41b64b7aae67bec01f30649c7841b239055323))
* **widgets:** show a red crossed globe when the network is not connected ([5de939e](https://github.com/non7top/justthebar/commit/5de939e0cc2ab9432742b69cecf5d94acb894528))
