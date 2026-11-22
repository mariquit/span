# Third-party notices

`span` includes or downloads components maintained by other projects.
Their copyrights remain with their respective owners.

## all-MiniLM-L6-v2 ONNX model

- Repository: <https://huggingface.co/Qdrant/all-MiniLM-L6-v2-onnx>
- Pinned revision: `d13954661f83248295ba75c1ed411eef3b7b936e`
- Upstream model: <https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2>
- License: Apache License 2.0

The model is downloaded separately on first use and is not included in the
source repository or release binary.

## Bekko Static Embedding v1

- Repository: <https://huggingface.co/hotchpotch/bekko-static-embedding-v1>
- Author: Yuichi Tateno (`hotchpotch`)
- Pinned revision: `c18167b8f244c1bc6de21ce9b710bac8a29c4615`
- License: MIT, as stated in the model card
- Files: `tokenizer.json` and `onnx/model_int8_dim512.onnx`

Span downloads these files separately only when Bekko is selected. The model
and tokenizer are not included in the source repository or release binary.

## ONNX Runtime

- Project: <https://onnxruntime.ai/>
- License: MIT

ONNX Runtime is used through the `ort` and `fastembed` Rust crates.

## Rust dependencies

The complete dependency graph and exact versions are recorded in `Cargo.lock`.
Release binaries are built from that lockfile.
