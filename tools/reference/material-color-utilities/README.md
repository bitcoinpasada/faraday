# material-color-utilities, vendored

Google's implementation of Material 3's colour system (HCT, tonal
palettes, dynamic colour, contrast curves), the TypeScript port as
published to npm, copied unchanged. It makes Faraday's colour tables:
`tools/themes/gen.mjs` builds every theme's schemes with it and writes
`faraday/faraday-ui/src/pal/schemes.rs` (`docs/THEMES.md` §10). Faraday
ships the tables, not this code.

Source: the npm package `@material/material-color-utilities` 0.4.0,
`https://registry.npmjs.org/@material/material-color-utilities/-/material-color-utilities-0.4.0.tgz`,
SHA-256 `2f0b759749846357dd2d9ce0e1d5e8071e5b51bda24178b3d24ba75afa7f723d`.
Built from `https://github.com/material-foundation/material-color-utilities`,
folder `typescript/`, commit `eeaf82b8e11bf20f6d8da7c76336575b69e79e01`
(2026-01-20, the package's recorded `gitHead`). Apache-2.0, `LICENSE`.

What is here: the package's 45 `.js` files, its `package.json` and its
`LICENSE`, byte for byte, at their paths in the package. Left out: the
type declarations (`.d.ts`), the source maps (`.js.map`) and the
package's `README.md`, which nothing runs. The published files are plain
JavaScript with no dependencies, so they run under Node with nothing
fetched.

`tools/themes/resolve.mjs` is written here, not upstream: ten of the
published files import a sibling without its `.js` extension (written
for bundlers), which Node's resolver refuses. The hook adds the
extension to a relative import that lacks one, so these files stay as
published.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/themes/reference.sha256`, which
`tools/lint-themes.sh` checks before it runs the generator.

| File | SHA-256 |
|---|---|
| `LICENSE` | `8ded460ad5bd3ab9082eaa2f7502cc9bb621eaff87cecc55d823f21758707cea` |
| `blend/blend.js` | `56ec54b23dde3f0b69248b62864ddb5485566e3033d9230f5bc8dd3f0db009a9` |
| `contrast/contrast.js` | `0420e46808e0fa9f3f9f0e9bd3fbc9c9000c81e7a52a41c85dda6b5926b69a63` |
| `dislike/dislike_analyzer.js` | `6af868d67b5930d1615f2cebd0106dcf3a5783c4ec00317def86ffff83bb7ef6` |
| `dynamiccolor/color_spec.js` | `4903821f29c09e927d5627c49bced8dad7d78b80aacc84c26f2b3f1b2cf3a017` |
| `dynamiccolor/color_spec_2021.js` | `1af02c6835860f89b00c2ee575f5c8d1526509a8d655938842a775b89c25d66a` |
| `dynamiccolor/color_spec_2025.js` | `c640d8798097818896cde40666112e7f23f8c6fc9ce5a333bbf0e4cc7287150d` |
| `dynamiccolor/contrast_curve.js` | `554937ee9d1d2b8173cebed71a5f39c9afd33e057066851c674e126e35eba75d` |
| `dynamiccolor/dynamic_color.js` | `69af0944314b85621e4851911bc8efbed6195ef51d7f9b8ad24633451b2d48b1` |
| `dynamiccolor/dynamic_scheme.js` | `b3eb513e45058abe661afe9b986009b2a51fdb68b6fa84385c8ad7eb98f36a01` |
| `dynamiccolor/material_dynamic_colors.js` | `b483d28900ae44ca6485f671a53c7e61b8d52fc4dd2437fbb5abf747f5c37f6a` |
| `dynamiccolor/tone_delta_pair.js` | `a0cb62f3ad5d49b2c601768d2f2845b7c248ca2bf346216ff6d93e7fc7f62d78` |
| `dynamiccolor/variant.js` | `3844adb3fa2b5813a9a238a3caa428acc5fa0d80d46476cb267d08755be423aa` |
| `hct/cam16.js` | `199bd3e3d1735d8767e95dec83a6d84325a4e0240525e79d12d85a88499a57be` |
| `hct/hct.js` | `76f4cea3428c63429fffd15598c82f1d22a1488c1d06f271f4b61a7a0622bcb4` |
| `hct/hct_solver.js` | `162aa706a44121049394de1eceff5b95792477e7d64dbc3e3bc780fe33bc58e5` |
| `hct/viewing_conditions.js` | `82716556e424d0d22cdb7494db6a547cec806a88737303e9eb11f068e90afe9c` |
| `index.js` | `583c9caa2160a521cdbd3f482e5113843cba6dc14a08b5f8a1ce37bc12255fca` |
| `package.json` | `3f2d23d7d162010f2c08ba5d1d6625d39c154baa1be4e978f3e2364a980e679a` |
| `palettes/core_palette.js` | `f3292031bb28acdd2f887b37aa0a673a13311a9f0eeae776ad9442240507f6a0` |
| `palettes/core_palettes.js` | `31ce3b93fdb8c0534d9e55bc279e3a77f7422c60ff5aaae1dd3dbcaa5e24ef20` |
| `palettes/tonal_palette.js` | `a77531315d2685e42d1f66a00bf11d0d48d56b56853d4e0b983ba32c08cb707d` |
| `quantize/lab_point_provider.js` | `7232dc5a8a619214fac912c0232b4646f8a0a1d54da057e8ecda69c0e860cb0a` |
| `quantize/point_provider.js` | `b4754276a59446c73eb8aa7af12eb2fddfa046b855f535ce83642d658a835843` |
| `quantize/quantizer_celebi.js` | `6fa2a197e1e4824123f8d739936977e2a3079f857ab9b5e12af24875b4445388` |
| `quantize/quantizer_map.js` | `55d031b1ed30acd31be1e7d8f7b2e2705791e1453cc97bbb48531a9b227ff506` |
| `quantize/quantizer_wsmeans.js` | `ff4eb7aee23fc649d6cb067881afb2ccbfa5cdbda93bba7696d0b9b77a4dcf15` |
| `quantize/quantizer_wu.js` | `b84326943d35f4000f68fbf940018c505440bdf4ec15602a3226bc4b2b95f33f` |
| `scheme/scheme.js` | `82c0316b4f0de731efa9106359c3ab25f69cec4d43623a6f7b8160b985b73974` |
| `scheme/scheme_android.js` | `fdf95787f48a7ff3a8be7ecd6eacfe40dd2490925c877418c182dee99b1657b1` |
| `scheme/scheme_content.js` | `59316a5ca960f63145fbc7089bbcf311d9d32a31c859a4b060bc3153bdac28db` |
| `scheme/scheme_expressive.js` | `61f3b3e2a07bc80bf8472b9d8368d61c30979324dbbaa2643892e85bf072718c` |
| `scheme/scheme_fidelity.js` | `8ab0c333db04ffc146aaa7e2a207a26dab7908edf8c487fccd73a7c915864b18` |
| `scheme/scheme_fruit_salad.js` | `738f97605381bdc5328d6277bba72e09d494fb4e385a208add1a352dcdb7af52` |
| `scheme/scheme_monochrome.js` | `f32a65313bf78106a0ccbd04685f1d6c6f40e830ce7677485b43fbfcb37339c7` |
| `scheme/scheme_neutral.js` | `13ad487489bbbdbc0802c74e18d817ae84e0308e3f255dc70034646fb2842ae8` |
| `scheme/scheme_rainbow.js` | `4d2003bef585ae4123bb2ba83a3f777acf750f02821390204d722d76cfed1f7a` |
| `scheme/scheme_tonal_spot.js` | `a373b1b5d369efd870e370fd305cf240ad643cf1db98e566b3525b5fda0f570c` |
| `scheme/scheme_vibrant.js` | `5b426e3e678b0988951fc3cfff36821065bdfbf92a4e2f3e2a600b50d1a25854` |
| `score/score.js` | `916e5a770e8b5df8084e9fc3c7ccdebebc7777b170880b22fc2214f68f591029` |
| `temperature/temperature_cache.js` | `2af4caa641672fd2d29f8833ee4368fd16d51a66069666ab18d44aa5c33ac75c` |
| `utils/color_utils.js` | `bcfaaaafcea56c4681b3f1fd030a1791fe86d7b0322c6dbdfc6f9f4bf61b0aac` |
| `utils/image_utils.js` | `01b978561e8383eb78c68825a092dfbea8bd39062576a25c292c62d68984d217` |
| `utils/math_utils.js` | `81ade2fa60e4ce9c64e8fe47d15115ebc9191dd2f8efb5226bcdb5ec966f251e` |
| `utils/string_utils.js` | `4d45d3b14a0a7dc3ec6b5f480cf6a19b44f3f64c7655d16ac219b985c445f23f` |
| `utils/test_utils.js` | `834a62ac1aad8a11f979346a25ee44890ca75d0488dde856ace373a81ec1cc90` |
| `utils/theme_utils.js` | `9b0de447a1eccdae732a74d5d35b5dea7a337895257ef874ccf6684bf6cea4c8` |
