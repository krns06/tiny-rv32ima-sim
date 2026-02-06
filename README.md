# tiny-rv32ima-sim
rv32imaのriscvシミュレータ
![demo](demo.gif)

## Goals
- [X] OpenSBIのブート
- [X] Linuxカーネルをブート
- [X] Busyboxのシェルで遊べる

## Usage
1. OpenSBIをrv32ima向けにビルド
2. Linuxカーネル、Busyboxをrv32ima_zicntr_zicsr_zifencei_svaduをサポートするようにビルド
3. デバイスツリーソース(platform.dts)をビルドしdtbpに変換
4. src/main.rsの内容を適切に変更
6. 以下を実行
```bash
$ cargo r --release
```

