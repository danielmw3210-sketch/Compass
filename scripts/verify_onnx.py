import onnxruntime as ort
import numpy as np

def verify_model():
    model_path = "models/price_decision_v1.onnx"
    try:
        session = ort.InferenceSession(model_path)
    except Exception as e:
        print(f"Failed to load model: {e}")
        return

    input_name = session.get_inputs()[0].name
    
    # Test Input 1: 50000.0
    val1 = np.array([[50000.0]], dtype=np.float32)
    res1 = session.run(None, {input_name: val1})[0]
    
    # Test Input 2: 100000.0
    val2 = np.array([[100000.0]], dtype=np.float32)
    res2 = session.run(None, {input_name: val2})[0]
    
    print(f"Input: 50000.0 -> Output: {res1}")
    print(f"Input: 100000.0 -> Output: {res2}")
    
    if np.allclose(res1, res2):
        print("❌ MODEL FAILURE: Outputs are identical! Model is creating constant predictions.")
    else:
        print("✅ MODEL OK: Outputs vary with input.")

if __name__ == "__main__":
    verify_model()
