use crate::minimize::MinimizeMode;
use crate::pla as pla_format;

fn verify_example(name: &str, text: &str) {
    let pla = pla_format::Pla::parse(text)
        .unwrap_or_else(|error| panic!("{name} should parse as supported binary PLA: {error}"));
    assert!(pla.input_count > 0, "{name} should declare at least one input");
    assert!(pla.output_count > 0, "{name} should declare at least one output");
    assert!(
        !pla.f.cubes.is_empty() || !pla.d.cubes.is_empty() || !pla.r.cubes.is_empty(),
        "{name} should contain at least one ON, DC, or OFF cube"
    );

    let round_tripped = pla_format::Pla::parse(&pla.write(pla_format::F_TYPE))
        .unwrap_or_else(|error| panic!("{name} should round-trip through PLA writer: {error}"));
    assert_eq!(round_tripped.input_count, pla.input_count, "{name} input count changed");
    assert_eq!(round_tripped.output_count, pla.output_count, "{name} output count changed");
}

fn verify_minimized_example(name: &str, text: &str) {
    let original = crate::complete_binary_off_set(
        pla_format::Pla::parse(text)
            .unwrap_or_else(|error| panic!("{name} should parse as supported binary PLA: {error}")),
    );
    let minimized_text = crate::minimize_pla_text(text, MinimizeMode::FastJoint)
        .unwrap_or_else(|error| panic!("{name} should minimize with native Rust: {error}"));
    let minimized = pla_format::Pla::parse(&minimized_text)
        .unwrap_or_else(|error| panic!("{name} minimized PLA should parse: {error}"));
    let report = original.verify_minimized(&minimized);
    assert!(
        report.is_equivalent(),
        "{name} minimized cover should be equivalent: {report:?}"
    );
}

#[test]
fn parser_accepts_split_input_output_lines() {
    pla_format::Pla::parse(include_str!("../../LogicSynthesis/espresso/examples/indust/accpla"))
        .expect("accpla uses input and output cube fields on separate lines");
}

#[test]
fn parser_accepts_split_cube_tokens() {
    pla_format::Pla::parse(include_str!("../../LogicSynthesis/espresso/examples/indust/amd"))
        .expect("amd splits cube fields across multiple whitespace tokens");
}

#[test]
fn indust_alu2_fast_joint_returns_equivalent_cover() {
    verify_minimized_example(
        "examples/indust/alu2",
        include_str!("../../LogicSynthesis/espresso/examples/indust/alu2"),
    );
}

#[test]
fn indust_accpla() {
    verify_example("examples/indust/accpla", include_str!("../../LogicSynthesis/espresso/examples/indust/accpla"));
}

#[test]
fn indust_al2() {
    verify_example("examples/indust/al2", include_str!("../../LogicSynthesis/espresso/examples/indust/al2"));
}

#[test]
fn indust_alcom() {
    verify_example("examples/indust/alcom", include_str!("../../LogicSynthesis/espresso/examples/indust/alcom"));
}

#[test]
fn indust_alu1() {
    verify_example("examples/indust/alu1", include_str!("../../LogicSynthesis/espresso/examples/indust/alu1"));
}

#[test]
fn indust_alu2() {
    verify_example("examples/indust/alu2", include_str!("../../LogicSynthesis/espresso/examples/indust/alu2"));
}

#[test]
fn indust_alu3() {
    verify_example("examples/indust/alu3", include_str!("../../LogicSynthesis/espresso/examples/indust/alu3"));
}

#[test]
fn indust_amd() {
    verify_example("examples/indust/amd", include_str!("../../LogicSynthesis/espresso/examples/indust/amd"));
}

#[test]
fn indust_apla() {
    verify_example("examples/indust/apla", include_str!("../../LogicSynthesis/espresso/examples/indust/apla"));
}

#[test]
fn indust_b10() {
    verify_example("examples/indust/b10", include_str!("../../LogicSynthesis/espresso/examples/indust/b10"));
}

#[test]
fn indust_b11() {
    verify_example("examples/indust/b11", include_str!("../../LogicSynthesis/espresso/examples/indust/b11"));
}

#[test]
fn indust_b12() {
    verify_example("examples/indust/b12", include_str!("../../LogicSynthesis/espresso/examples/indust/b12"));
}

#[test]
fn indust_b2() {
    verify_example("examples/indust/b2", include_str!("../../LogicSynthesis/espresso/examples/indust/b2"));
}

#[test]
fn indust_b3() {
    verify_example("examples/indust/b3", include_str!("../../LogicSynthesis/espresso/examples/indust/b3"));
}

#[test]
fn indust_b4() {
    verify_example("examples/indust/b4", include_str!("../../LogicSynthesis/espresso/examples/indust/b4"));
}

#[test]
fn indust_b7() {
    verify_example("examples/indust/b7", include_str!("../../LogicSynthesis/espresso/examples/indust/b7"));
}

#[test]
fn indust_b9() {
    verify_example("examples/indust/b9", include_str!("../../LogicSynthesis/espresso/examples/indust/b9"));
}

#[test]
fn indust_bc0() {
    verify_example("examples/indust/bc0", include_str!("../../LogicSynthesis/espresso/examples/indust/bc0"));
}

#[test]
fn indust_bca() {
    verify_example("examples/indust/bca", include_str!("../../LogicSynthesis/espresso/examples/indust/bca"));
}

#[test]
fn indust_bcb() {
    verify_example("examples/indust/bcb", include_str!("../../LogicSynthesis/espresso/examples/indust/bcb"));
}

#[test]
fn indust_bcc() {
    verify_example("examples/indust/bcc", include_str!("../../LogicSynthesis/espresso/examples/indust/bcc"));
}

#[test]
fn indust_bcd() {
    verify_example("examples/indust/bcd", include_str!("../../LogicSynthesis/espresso/examples/indust/bcd"));
}

#[test]
fn indust_br1() {
    verify_example("examples/indust/br1", include_str!("../../LogicSynthesis/espresso/examples/indust/br1"));
}

#[test]
fn indust_br2() {
    verify_example("examples/indust/br2", include_str!("../../LogicSynthesis/espresso/examples/indust/br2"));
}

#[test]
fn indust_chkn() {
    verify_example("examples/indust/chkn", include_str!("../../LogicSynthesis/espresso/examples/indust/chkn"));
}

#[test]
fn indust_clpl() {
    verify_example("examples/indust/clpl", include_str!("../../LogicSynthesis/espresso/examples/indust/clpl"));
}

#[test]
fn indust_cps() {
    verify_example("examples/indust/cps", include_str!("../../LogicSynthesis/espresso/examples/indust/cps"));
}

#[test]
fn indust_dc1() {
    verify_example("examples/indust/dc1", include_str!("../../LogicSynthesis/espresso/examples/indust/dc1"));
}

#[test]
fn indust_dc2() {
    verify_example("examples/indust/dc2", include_str!("../../LogicSynthesis/espresso/examples/indust/dc2"));
}

#[test]
fn indust_dekoder() {
    verify_example("examples/indust/dekoder", include_str!("../../LogicSynthesis/espresso/examples/indust/dekoder"));
}

#[test]
fn indust_dk17() {
    verify_example("examples/indust/dk17", include_str!("../../LogicSynthesis/espresso/examples/indust/dk17"));
}

#[test]
fn indust_dk27() {
    verify_example("examples/indust/dk27", include_str!("../../LogicSynthesis/espresso/examples/indust/dk27"));
}

#[test]
fn indust_dk48() {
    verify_example("examples/indust/dk48", include_str!("../../LogicSynthesis/espresso/examples/indust/dk48"));
}

#[test]
fn indust_ex4() {
    verify_example("examples/indust/ex4", include_str!("../../LogicSynthesis/espresso/examples/indust/ex4"));
}

#[test]
fn indust_ex5() {
    verify_example("examples/indust/ex5", include_str!("../../LogicSynthesis/espresso/examples/indust/ex5"));
}

#[test]
fn indust_ex7() {
    verify_example("examples/indust/ex7", include_str!("../../LogicSynthesis/espresso/examples/indust/ex7"));
}

#[test]
fn indust_exep() {
    verify_example("examples/indust/exep", include_str!("../../LogicSynthesis/espresso/examples/indust/exep"));
}

#[test]
fn indust_exp() {
    verify_example("examples/indust/exp", include_str!("../../LogicSynthesis/espresso/examples/indust/exp"));
}

#[test]
fn indust_exps() {
    verify_example("examples/indust/exps", include_str!("../../LogicSynthesis/espresso/examples/indust/exps"));
}

#[test]
fn indust_gary() {
    verify_example("examples/indust/gary", include_str!("../../LogicSynthesis/espresso/examples/indust/gary"));
}

#[test]
fn indust_ibm() {
    verify_example("examples/indust/ibm", include_str!("../../LogicSynthesis/espresso/examples/indust/ibm"));
}

#[test]
fn indust_in0() {
    verify_example("examples/indust/in0", include_str!("../../LogicSynthesis/espresso/examples/indust/in0"));
}

#[test]
fn indust_in1() {
    verify_example("examples/indust/in1", include_str!("../../LogicSynthesis/espresso/examples/indust/in1"));
}

#[test]
fn indust_in2() {
    verify_example("examples/indust/in2", include_str!("../../LogicSynthesis/espresso/examples/indust/in2"));
}

#[test]
fn indust_in3() {
    verify_example("examples/indust/in3", include_str!("../../LogicSynthesis/espresso/examples/indust/in3"));
}

#[test]
fn indust_in4() {
    verify_example("examples/indust/in4", include_str!("../../LogicSynthesis/espresso/examples/indust/in4"));
}

#[test]
fn indust_in5() {
    verify_example("examples/indust/in5", include_str!("../../LogicSynthesis/espresso/examples/indust/in5"));
}

#[test]
fn indust_in6() {
    verify_example("examples/indust/in6", include_str!("../../LogicSynthesis/espresso/examples/indust/in6"));
}

#[test]
fn indust_in7() {
    verify_example("examples/indust/in7", include_str!("../../LogicSynthesis/espresso/examples/indust/in7"));
}

#[test]
fn indust_inc() {
    verify_example("examples/indust/inc", include_str!("../../LogicSynthesis/espresso/examples/indust/inc"));
}

#[test]
fn indust_intb() {
    verify_example("examples/indust/intb", include_str!("../../LogicSynthesis/espresso/examples/indust/intb"));
}

#[test]
fn indust_jbp() {
    verify_example("examples/indust/jbp", include_str!("../../LogicSynthesis/espresso/examples/indust/jbp"));
}

#[test]
fn indust_lin_rom() {
    verify_example("examples/indust/lin.rom", include_str!("../../LogicSynthesis/espresso/examples/indust/lin.rom"));
}

#[test]
fn indust_luc() {
    verify_example("examples/indust/luc", include_str!("../../LogicSynthesis/espresso/examples/indust/luc"));
}

#[test]
fn indust_m1() {
    verify_example("examples/indust/m1", include_str!("../../LogicSynthesis/espresso/examples/indust/m1"));
}

#[test]
fn indust_m2() {
    verify_example("examples/indust/m2", include_str!("../../LogicSynthesis/espresso/examples/indust/m2"));
}

#[test]
fn indust_m3() {
    verify_example("examples/indust/m3", include_str!("../../LogicSynthesis/espresso/examples/indust/m3"));
}

#[test]
fn indust_m4() {
    verify_example("examples/indust/m4", include_str!("../../LogicSynthesis/espresso/examples/indust/m4"));
}

#[test]
fn indust_mainpla() {
    verify_example("examples/indust/mainpla", include_str!("../../LogicSynthesis/espresso/examples/indust/mainpla"));
}

#[test]
fn indust_mark1() {
    verify_example("examples/indust/mark1", include_str!("../../LogicSynthesis/espresso/examples/indust/mark1"));
}

#[test]
fn indust_max1024() {
    verify_example("examples/indust/max1024", include_str!("../../LogicSynthesis/espresso/examples/indust/max1024"));
}

#[test]
fn indust_max128() {
    verify_example("examples/indust/max128", include_str!("../../LogicSynthesis/espresso/examples/indust/max128"));
}

#[test]
fn indust_max46() {
    verify_example("examples/indust/max46", include_str!("../../LogicSynthesis/espresso/examples/indust/max46"));
}

#[test]
fn indust_max512() {
    verify_example("examples/indust/max512", include_str!("../../LogicSynthesis/espresso/examples/indust/max512"));
}

#[test]
fn indust_misg() {
    verify_example("examples/indust/misg", include_str!("../../LogicSynthesis/espresso/examples/indust/misg"));
}

#[test]
fn indust_mish() {
    verify_example("examples/indust/mish", include_str!("../../LogicSynthesis/espresso/examples/indust/mish"));
}

#[test]
fn indust_misj() {
    verify_example("examples/indust/misj", include_str!("../../LogicSynthesis/espresso/examples/indust/misj"));
}

#[test]
fn indust_mp2d() {
    verify_example("examples/indust/mp2d", include_str!("../../LogicSynthesis/espresso/examples/indust/mp2d"));
}

#[test]
fn indust_newapla() {
    verify_example("examples/indust/newapla", include_str!("../../LogicSynthesis/espresso/examples/indust/newapla"));
}

#[test]
fn indust_newapla1() {
    verify_example("examples/indust/newapla1", include_str!("../../LogicSynthesis/espresso/examples/indust/newapla1"));
}

#[test]
fn indust_newapla2() {
    verify_example("examples/indust/newapla2", include_str!("../../LogicSynthesis/espresso/examples/indust/newapla2"));
}

#[test]
fn indust_newbyte() {
    verify_example("examples/indust/newbyte", include_str!("../../LogicSynthesis/espresso/examples/indust/newbyte"));
}

#[test]
fn indust_newcond() {
    verify_example("examples/indust/newcond", include_str!("../../LogicSynthesis/espresso/examples/indust/newcond"));
}

#[test]
fn indust_newcpla1() {
    verify_example("examples/indust/newcpla1", include_str!("../../LogicSynthesis/espresso/examples/indust/newcpla1"));
}

#[test]
fn indust_newcpla2() {
    verify_example("examples/indust/newcpla2", include_str!("../../LogicSynthesis/espresso/examples/indust/newcpla2"));
}

#[test]
fn indust_newcwp() {
    verify_example("examples/indust/newcwp", include_str!("../../LogicSynthesis/espresso/examples/indust/newcwp"));
}

#[test]
fn indust_newill() {
    verify_example("examples/indust/newill", include_str!("../../LogicSynthesis/espresso/examples/indust/newill"));
}

#[test]
fn indust_newtag() {
    verify_example("examples/indust/newtag", include_str!("../../LogicSynthesis/espresso/examples/indust/newtag"));
}

#[test]
fn indust_newtpla() {
    verify_example("examples/indust/newtpla", include_str!("../../LogicSynthesis/espresso/examples/indust/newtpla"));
}

#[test]
fn indust_newtpla1() {
    verify_example("examples/indust/newtpla1", include_str!("../../LogicSynthesis/espresso/examples/indust/newtpla1"));
}

#[test]
fn indust_newtpla2() {
    verify_example("examples/indust/newtpla2", include_str!("../../LogicSynthesis/espresso/examples/indust/newtpla2"));
}

#[test]
fn indust_newxcpla1() {
    verify_example("examples/indust/newxcpla1", include_str!("../../LogicSynthesis/espresso/examples/indust/newxcpla1"));
}

#[test]
fn indust_opa() {
    verify_example("examples/indust/opa", include_str!("../../LogicSynthesis/espresso/examples/indust/opa"));
}

#[test]
fn indust_p82() {
    verify_example("examples/indust/p82", include_str!("../../LogicSynthesis/espresso/examples/indust/p82"));
}

#[test]
fn indust_pdc() {
    verify_example("examples/indust/pdc", include_str!("../../LogicSynthesis/espresso/examples/indust/pdc"));
}

#[test]
fn indust_pope_rom() {
    verify_example("examples/indust/pope.rom", include_str!("../../LogicSynthesis/espresso/examples/indust/pope.rom"));
}

#[test]
fn indust_prom1() {
    verify_example("examples/indust/prom1", include_str!("../../LogicSynthesis/espresso/examples/indust/prom1"));
}

#[test]
fn indust_prom2() {
    verify_example("examples/indust/prom2", include_str!("../../LogicSynthesis/espresso/examples/indust/prom2"));
}

#[test]
fn indust_risc() {
    verify_example("examples/indust/risc", include_str!("../../LogicSynthesis/espresso/examples/indust/risc"));
}

#[test]
fn indust_ryy6() {
    verify_example("examples/indust/ryy6", include_str!("../../LogicSynthesis/espresso/examples/indust/ryy6"));
}

#[test]
fn indust_sex() {
    verify_example("examples/indust/sex", include_str!("../../LogicSynthesis/espresso/examples/indust/sex"));
}

#[test]
fn indust_shift() {
    verify_example("examples/indust/shift", include_str!("../../LogicSynthesis/espresso/examples/indust/shift"));
}

#[test]
fn indust_signet() {
    verify_example("examples/indust/signet", include_str!("../../LogicSynthesis/espresso/examples/indust/signet"));
}

#[test]
fn indust_soar_pla() {
    verify_example("examples/indust/soar.pla", include_str!("../../LogicSynthesis/espresso/examples/indust/soar.pla"));
}

#[test]
fn indust_spla() {
    verify_example("examples/indust/spla", include_str!("../../LogicSynthesis/espresso/examples/indust/spla"));
}

#[test]
fn indust_sqn() {
    verify_example("examples/indust/sqn", include_str!("../../LogicSynthesis/espresso/examples/indust/sqn"));
}

#[test]
fn indust_t1() {
    verify_example("examples/indust/t1", include_str!("../../LogicSynthesis/espresso/examples/indust/t1"));
}

#[test]
fn indust_t2() {
    verify_example("examples/indust/t2", include_str!("../../LogicSynthesis/espresso/examples/indust/t2"));
}

#[test]
fn indust_t3() {
    verify_example("examples/indust/t3", include_str!("../../LogicSynthesis/espresso/examples/indust/t3"));
}

#[test]
fn indust_t4() {
    verify_example("examples/indust/t4", include_str!("../../LogicSynthesis/espresso/examples/indust/t4"));
}

#[test]
fn indust_ti() {
    verify_example("examples/indust/ti", include_str!("../../LogicSynthesis/espresso/examples/indust/ti"));
}

#[test]
fn indust_tms() {
    verify_example("examples/indust/tms", include_str!("../../LogicSynthesis/espresso/examples/indust/tms"));
}

#[test]
fn indust_ts10() {
    verify_example("examples/indust/ts10", include_str!("../../LogicSynthesis/espresso/examples/indust/ts10"));
}

#[test]
fn indust_vg2() {
    verify_example("examples/indust/vg2", include_str!("../../LogicSynthesis/espresso/examples/indust/vg2"));
}

#[test]
fn indust_vtx1() {
    verify_example("examples/indust/vtx1", include_str!("../../LogicSynthesis/espresso/examples/indust/vtx1"));
}

#[test]
fn indust_wim() {
    verify_example("examples/indust/wim", include_str!("../../LogicSynthesis/espresso/examples/indust/wim"));
}

#[test]
fn indust_x1dn() {
    verify_example("examples/indust/x1dn", include_str!("../../LogicSynthesis/espresso/examples/indust/x1dn"));
}

#[test]
fn indust_x2dn() {
    verify_example("examples/indust/x2dn", include_str!("../../LogicSynthesis/espresso/examples/indust/x2dn"));
}

#[test]
fn indust_x6dn() {
    verify_example("examples/indust/x6dn", include_str!("../../LogicSynthesis/espresso/examples/indust/x6dn"));
}

#[test]
fn indust_x7dn() {
    verify_example("examples/indust/x7dn", include_str!("../../LogicSynthesis/espresso/examples/indust/x7dn"));
}

#[test]
fn indust_x9dn() {
    verify_example("examples/indust/x9dn", include_str!("../../LogicSynthesis/espresso/examples/indust/x9dn"));
}

#[test]
fn indust_xparc() {
    verify_example("examples/indust/xparc", include_str!("../../LogicSynthesis/espresso/examples/indust/xparc"));
}

#[test]
fn math_add6() {
    verify_example("examples/math/add6", include_str!("../../LogicSynthesis/espresso/examples/math/add6"));
}

#[test]
fn math_addm4() {
    verify_example("examples/math/addm4", include_str!("../../LogicSynthesis/espresso/examples/math/addm4"));
}

#[test]
fn math_adr4() {
    verify_example("examples/math/adr4", include_str!("../../LogicSynthesis/espresso/examples/math/adr4"));
}

#[test]
fn math_bcd_div3() {
    verify_example("examples/math/bcd.div3", include_str!("../../LogicSynthesis/espresso/examples/math/bcd.div3"));
}

#[test]
fn math_co14() {
    verify_example("examples/math/co14", include_str!("../../LogicSynthesis/espresso/examples/math/co14"));
}

#[test]
fn math_dist() {
    verify_example("examples/math/dist", include_str!("../../LogicSynthesis/espresso/examples/math/dist"));
}

#[test]
fn math_f51m() {
    verify_example("examples/math/f51m", include_str!("../../LogicSynthesis/espresso/examples/math/f51m"));
}

#[test]
fn math_l8err() {
    verify_example("examples/math/l8err", include_str!("../../LogicSynthesis/espresso/examples/math/l8err"));
}

#[test]
fn math_life() {
    verify_example("examples/math/life", include_str!("../../LogicSynthesis/espresso/examples/math/life"));
}

#[test]
fn math_log8mod() {
    verify_example("examples/math/log8mod", include_str!("../../LogicSynthesis/espresso/examples/math/log8mod"));
}

#[test]
fn math_m181() {
    verify_example("examples/math/m181", include_str!("../../LogicSynthesis/espresso/examples/math/m181"));
}

#[test]
fn math_mlp4() {
    verify_example("examples/math/mlp4", include_str!("../../LogicSynthesis/espresso/examples/math/mlp4"));
}

#[test]
fn math_radd() {
    verify_example("examples/math/radd", include_str!("../../LogicSynthesis/espresso/examples/math/radd"));
}

#[test]
fn math_rckl() {
    verify_example("examples/math/rckl", include_str!("../../LogicSynthesis/espresso/examples/math/rckl"));
}

#[test]
fn math_rd53() {
    verify_example("examples/math/rd53", include_str!("../../LogicSynthesis/espresso/examples/math/rd53"));
}

#[test]
fn math_rd73() {
    verify_example("examples/math/rd73", include_str!("../../LogicSynthesis/espresso/examples/math/rd73"));
}

#[test]
fn math_root() {
    verify_example("examples/math/root", include_str!("../../LogicSynthesis/espresso/examples/math/root"));
}

#[test]
fn math_sqr6() {
    verify_example("examples/math/sqr6", include_str!("../../LogicSynthesis/espresso/examples/math/sqr6"));
}

#[test]
fn math_sym10() {
    verify_example("examples/math/sym10", include_str!("../../LogicSynthesis/espresso/examples/math/sym10"));
}

#[test]
fn math_tial() {
    verify_example("examples/math/tial", include_str!("../../LogicSynthesis/espresso/examples/math/tial"));
}

#[test]
fn math_z4() {
    verify_example("examples/math/z4", include_str!("../../LogicSynthesis/espresso/examples/math/z4"));
}

#[test]
fn math_z5xp1() {
    verify_example("examples/math/Z5xp1", include_str!("../../LogicSynthesis/espresso/examples/math/Z5xp1"));
}

#[test]
fn math_z9sym() {
    verify_example("examples/math/Z9sym", include_str!("../../LogicSynthesis/espresso/examples/math/Z9sym"));
}

#[test]
fn random_bench() {
    verify_example("examples/random/bench", include_str!("../../LogicSynthesis/espresso/examples/random/bench"));
}

#[test]
fn random_bench1() {
    verify_example("examples/random/bench1", include_str!("../../LogicSynthesis/espresso/examples/random/bench1"));
}

#[test]
fn random_ex1010() {
    verify_example("examples/random/ex1010", include_str!("../../LogicSynthesis/espresso/examples/random/ex1010"));
}

#[test]
fn random_exam() {
    verify_example("examples/random/exam", include_str!("../../LogicSynthesis/espresso/examples/random/exam"));
}

#[test]
fn random_fout() {
    verify_example("examples/random/fout", include_str!("../../LogicSynthesis/espresso/examples/random/fout"));
}

#[test]
fn random_p1() {
    verify_example("examples/random/p1", include_str!("../../LogicSynthesis/espresso/examples/random/p1"));
}

#[test]
fn random_p3() {
    verify_example("examples/random/p3", include_str!("../../LogicSynthesis/espresso/examples/random/p3"));
}

#[test]
fn random_test1() {
    verify_example("examples/random/test1", include_str!("../../LogicSynthesis/espresso/examples/random/test1"));
}

#[test]
fn random_test2() {
    verify_example("examples/random/test2", include_str!("../../LogicSynthesis/espresso/examples/random/test2"));
}

#[test]
fn random_test3() {
    verify_example("examples/random/test3", include_str!("../../LogicSynthesis/espresso/examples/random/test3"));
}

#[test]
fn random_test4() {
    verify_example("examples/random/test4", include_str!("../../LogicSynthesis/espresso/examples/random/test4"));
}
