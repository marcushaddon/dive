#!/usr/bin/env sh
cargo xtask bundle whammy --release
rm -rf ~/Library/Audio/Plug-Ins/VST3/Whammy.vst3
cp -R ./target/bundled/Whammy.vst3 ~/Library/Audio/Plug-Ins/VST3/Whammy.vst3 