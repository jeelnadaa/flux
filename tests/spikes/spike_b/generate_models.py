"""Generates ONNX test models for Spike B: small RNNoise-class model and large HTDemucs-class chunked model."""

import numpy as np
import onnx
from onnx import helper, numpy_helper, TensorProto
from pathlib import Path


def create_small_rnnoise_model(output_path: Path):
    """Generates an RNNoise-class frame-based speech enhancement ONNX model (feature frame [1, 42] -> [1, 42])."""
    feat_dim = 42
    hidden_dim = 64

    # Inputs and outputs
    x = helper.make_tensor_value_info("input", TensorProto.FLOAT, [1, feat_dim])
    y = helper.make_tensor_value_info("output", TensorProto.FLOAT, [1, feat_dim])

    # Weights
    rng = np.random.RandomState(42)
    w1 = rng.randn(feat_dim, hidden_dim).astype(np.float32) * 0.1
    b1 = np.zeros(hidden_dim, dtype=np.float32)
    w2 = rng.randn(hidden_dim, feat_dim).astype(np.float32) * 0.1
    b2 = np.zeros(feat_dim, dtype=np.float32)

    init_w1 = numpy_helper.from_array(w1, "w1")
    init_b1 = numpy_helper.from_array(b1, "b1")
    init_w2 = numpy_helper.from_array(w2, "w2")
    init_b2 = numpy_helper.from_array(b2, "b2")

    # Nodes:
    # 1. Gemm(x, w1, b1) -> h1
    # 2. Relu(h1) -> h1_act
    # 3. Gemm(h1_act, w2, b2) -> mask_logits
    # 4. Sigmoid(mask_logits) -> mask
    # 5. Mul(x, mask) -> output
    n1 = helper.make_node("Gemm", ["input", "w1", "b1"], ["h1"], transA=0, transB=0)
    n2 = helper.make_node("Relu", ["h1"], ["h1_act"])
    n3 = helper.make_node("Gemm", ["h1_act", "w2", "b2"], ["mask_logits"], transA=0, transB=0)
    n4 = helper.make_node("Sigmoid", ["mask_logits"], ["mask"])
    n5 = helper.make_node("Mul", ["input", "mask"], ["output"])

    graph = helper.make_graph(
        [n1, n2, n3, n4, n5],
        "rnnoise_small_graph",
        [x],
        [y],
        [init_w1, init_b1, init_w2, init_b2],
    )
    model = helper.make_model(graph, ir_version=9, opset_imports=[helper.make_opsetid("", 17)])
    onnx.checker.check_model(model)
    onnx.save(model, str(output_path))
    print(f"Saved small model ({output_path.stat().st_size / 1024:.1f} KB) to {output_path}")


def create_large_htdemucs_model(output_path: Path):
    """Generates an HTDemucs-class chunked 4-stem separation model (input chunk [1, 2, 48000] -> output stems [1, 4, 2, 48000])."""
    # Audio chunk dimensions: batch=1, channels=2 (stereo), frames=48000 (1 second)
    in_channels = 2
    hidden_ch = 16
    stems = 4

    x = helper.make_tensor_value_info("audio_chunk", TensorProto.FLOAT, [1, in_channels, 48000])
    y = helper.make_tensor_value_info("separated_stems", TensorProto.FLOAT, [1, stems, in_channels, 48000])

    rng = np.random.RandomState(42)

    # Conv1: 2 -> 16 channels, kernel=7, pad=3
    w_conv1 = rng.randn(hidden_ch, in_channels, 7).astype(np.float32) * 0.05
    b_conv1 = np.zeros(hidden_ch, dtype=np.float32)

    # Conv2: 16 -> 16 channels, kernel=7, pad=3
    w_conv2 = rng.randn(hidden_ch, hidden_ch, 7).astype(np.float32) * 0.05
    b_conv2 = np.zeros(hidden_ch, dtype=np.float32)

    # Conv3 (stems projection): 16 -> 8 channels (4 stems x 2 ch), kernel=1
    out_ch_total = stems * in_channels  # 8
    w_conv3 = rng.randn(out_ch_total, hidden_ch, 1).astype(np.float32) * 0.05
    b_conv3 = np.zeros(out_ch_total, dtype=np.float32)

    # Shape tensor for Reshape to [1, 4, 2, 48000]
    reshape_shape = np.array([1, stems, in_channels, 48000], dtype=np.int64)

    init_w1 = numpy_helper.from_array(w_conv1, "w_conv1")
    init_b1 = numpy_helper.from_array(b_conv1, "b_conv1")
    init_w2 = numpy_helper.from_array(w_conv2, "w_conv2")
    init_b2 = numpy_helper.from_array(b_conv2, "b_conv2")
    init_w3 = numpy_helper.from_array(w_conv3, "w_conv3")
    init_b3 = numpy_helper.from_array(b_conv3, "b_conv3")
    init_shape = numpy_helper.from_array(reshape_shape, "reshape_shape")

    n1 = helper.make_node(
        "Conv", ["audio_chunk", "w_conv1", "b_conv1"], ["conv1_out"], kernel_shape=[7], pads=[3, 3]
    )
    n2 = helper.make_node("Relu", ["conv1_out"], ["act1_out"])
    n3 = helper.make_node(
        "Conv", ["act1_out", "w_conv2", "b_conv2"], ["conv2_out"], kernel_shape=[7], pads=[3, 3]
    )
    n4 = helper.make_node("Relu", ["conv2_out"], ["act2_out"])
    n5 = helper.make_node(
        "Conv", ["act2_out", "w_conv3", "b_conv3"], ["stems_flat"], kernel_shape=[1], pads=[0, 0]
    )
    n6 = helper.make_node("Reshape", ["stems_flat", "reshape_shape"], ["separated_stems"])

    graph = helper.make_graph(
        [n1, n2, n3, n4, n5, n6],
        "htdemucs_large_graph",
        [x],
        [y],
        [init_w1, init_b1, init_w2, init_b2, init_w3, init_b3, init_shape],
    )
    model = helper.make_model(graph, ir_version=9, opset_imports=[helper.make_opsetid("", 17)])
    onnx.checker.check_model(model)
    onnx.save(model, str(output_path))
    print(f"Saved large model ({output_path.stat().st_size / (1024*1024):.2f} MB) to {output_path}")


if __name__ == "__main__":
    out_dir = Path(__file__).resolve().parent
    create_small_rnnoise_model(out_dir / "small_model.onnx")
    create_large_htdemucs_model(out_dir / "large_model.onnx")
