use arm64jit::jit::{CpuState, exec_bytes};
fn main() {
    let mut st = CpuState::new();
    st.v[2] = 5; st.v[4] = 3;
    let code = [0x20,0xe0,0xe2,0x0e, 0xc0,0x03,0x5f,0xd6];
    let r = exec_bytes(&mut st, &code, 0);
    println!("res={:?} v0={:016x} v1={:016x}", r, st.v[0], st.v[1]);
}