/* 004225c6 FUN_004225c6 */

void __fastcall FUN_004225c6(int param_1)

{
  FUN_0043ebd0((uint *)(param_1 + 0x20e8),
               (uint *)
               "GATE zero0\t0\tO=CONST0;\nGATE one0\t0\tO=CONST1;\nGATE inv1\t1\tO=!a;\tPIN * INV 1 999 0.9 0.3 0.9 0.3\n"
              );
  _memset((void *)(param_1 + 0x3a8),0,0x12a8);
  *(undefined4 *)(param_1 + 0x3a8) = 0x11;
  *(undefined4 *)(param_1 + 0x3b4) = 0x403;
  *(undefined4 *)(param_1 + 0x3b8) = 0x3f5;
  *(undefined4 *)(param_1 + 0x3cc) = 1;
  FUN_0043ebd0((uint *)(param_1 + 0x3bc),(uint *)"nand2");
  FUN_0043ebd0((uint *)(param_1 + 0x3d0),
               (uint *)"GATE nand2\t2\tO=!(a*b);\tPIN * INV 1 999 1.0 0.2 1.0 0.2\n");
  *(undefined4 *)(param_1 + 0x22ec) = 0;
  *(undefined4 *)(param_1 + 0x4d0) = 0x3f6;
  FUN_0043ebd0((uint *)(param_1 + 0x4d4),(uint *)"nand3");
  FUN_0043ebd0((uint *)(param_1 + 0x4e8),
               (uint *)"GATE nand3\t3\tO=!(a*b*c);\tPIN * INV 1 999 1.1 0.3 1.1 0.3\n");
  *(undefined4 *)(param_1 + 0x22f0) = 1;
  *(undefined4 *)(param_1 + 0x5e8) = 0x3f7;
  FUN_0043ebd0((uint *)(param_1 + 0x5ec),(uint *)"nand4");
  FUN_0043ebd0((uint *)(param_1 + 0x600),
               (uint *)"GATE nand4\t4\tO=!(a*b*c*d);\tPIN * INV 1 999 1.4 0.4 1.4 0.4\n");
  *(undefined4 *)(param_1 + 0x22f4) = 2;
  *(undefined4 *)(param_1 + 0x700) = 0x3f8;
  *(undefined4 *)(param_1 + 0x714) = 1;
  FUN_0043ebd0((uint *)(param_1 + 0x704),(uint *)&DAT_0044d0a4);
  FUN_0043ebd0((uint *)(param_1 + 0x718),
               (uint *)"GATE nor2\t2\tO=!(a+b);\tPIN * INV 1 999 1.4 0.5 1.4 0.5\n");
  *(undefined4 *)(param_1 + 0x22f8) = 3;
  *(undefined4 *)(param_1 + 0x818) = 0x3f9;
  FUN_0043ebd0((uint *)(param_1 + 0x81c),(uint *)&DAT_0044d064);
  FUN_0043ebd0((uint *)(param_1 + 0x830),
               (uint *)"GATE nor3\t3\tO=!(a+b+c);\tPIN * INV 1 999 2.4 0.7 2.4 0.7\n");
  *(undefined4 *)(param_1 + 0x22fc) = 4;
  *(undefined4 *)(param_1 + 0x930) = 0x3fa;
  FUN_0043ebd0((uint *)(param_1 + 0x934),(uint *)&DAT_0044d020);
  FUN_0043ebd0((uint *)(param_1 + 0x948),
               (uint *)"GATE nor4\t4\tO=!(a+b+c+d);\tPIN * INV 1 999 3.8 1.0 3.8 1.0\n");
  *(undefined4 *)(param_1 + 0x2300) = 5;
  *(undefined4 *)(param_1 + 0xa48) = 0x3fb;
  FUN_0043ebd0((uint *)(param_1 + 0xa4c),(uint *)"exor2");
  FUN_0043ebd0((uint *)(param_1 + 0xa60),
               (uint *)"GATE exor2\t5.5\tO=a*!b+!a*b;\tPIN * UNKNOWN 2 999 1.9 0.5 1.9 0.5\n");
  *(undefined4 *)(param_1 + 0xb60) = 0x3fd;
  FUN_0043ebd0((uint *)(param_1 + 0xb64),(uint *)&DAT_0044cf90);
  FUN_0043ebd0((uint *)(param_1 + 0xb78),
               (uint *)"GATE and2\t2\tO=a*b;\tPIN * INV 1 999 1.0 0.2 1.0 0.2\n");
  *(undefined4 *)(param_1 + 0xc78) = 0x3fe;
  FUN_0043ebd0((uint *)(param_1 + 0xc7c),(uint *)&DAT_0044cf54);
  FUN_0043ebd0((uint *)(param_1 + 0xc90),
               (uint *)"GATE and3\t2\tO=a*b*c;\tPIN * INV 1 999 1.0 0.2 1.0 0.2\n");
  *(undefined4 *)(param_1 + 0xd90) = 0x3ff;
  FUN_0043ebd0((uint *)(param_1 + 0xd94),(uint *)&DAT_0044cf14);
  FUN_0043ebd0((uint *)(param_1 + 0xda8),
               (uint *)"GATE and4\t2\tO=a*b*c*d;\tPIN * INV 1 999 1.0 0.2 1.0 0.2\n");
  *(undefined4 *)(param_1 + 0xea8) = 0x400;
  FUN_0043ebd0((uint *)(param_1 + 0xeac),(uint *)&DAT_0044ced8);
  FUN_0043ebd0((uint *)(param_1 + 0xec0),
               (uint *)"GATE or2\t2\tO=a+b;\tPIN * INV 1 999 1.4 0.5 1.4 0.5\n");
  *(undefined4 *)(param_1 + 0xfc0) = 0x401;
  FUN_0043ebd0((uint *)(param_1 + 0xfc4),(uint *)&DAT_0044cea0);
  FUN_0043ebd0((uint *)(param_1 + 0xfd8),
               (uint *)"GATE or3\t3\tO=a+b+c;\tPIN * INV 1 999 2.4 0.7 2.4 0.7\n");
  *(undefined4 *)(param_1 + 0x10d8) = 0x402;
  FUN_0043ebd0((uint *)(param_1 + 0x10dc),(uint *)&DAT_0044ce64);
  FUN_0043ebd0((uint *)(param_1 + 0x10f0),
               (uint *)"GATE or4\t4\tO=a+b+c+d;\tPIN * INV 1 999 3.8 1.0 3.8 1.0\n");
  *(undefined4 *)(param_1 + 0x11f0) = 0x3fc;
  FUN_0043ebd0((uint *)(param_1 + 0x11f4),(uint *)&DAT_0044ce24);
  FUN_0043ebd0((uint *)(param_1 + 0x1208),
               (uint *)
               "GATE mux2\t4\tO=1D1*!3SEL+2D2*3SEL;\nPIN\t1D1 NONINV 1 999 1 .2 1 .2\nPIN\t2D2 NONINV 1 999 1 .2 1 .2\nPIN\t3SEL UNKNOWN 1 999 1 .2 1 .2\n"
              );
  *(undefined4 *)(param_1 + 0x1308) = 0x3f4;
  *(undefined4 *)(param_1 + 0x131c) = 1;
  FUN_0043ebd0((uint *)(param_1 + 0x130c),(uint *)&DAT_0044cbf8);
  FUN_0043ebd0((uint *)(param_1 + 0x1320),(uint *)&DAT_0044ad26);
  *(undefined4 *)(param_1 + 0x2304) = 0xe;
  *(undefined4 *)(param_1 + 0x1420) = 0x3fb;
  FUN_0043ebd0((uint *)(param_1 + 0x1424),(uint *)"exor2");
  FUN_0043ebd0((uint *)(param_1 + 0x1438),
               (uint *)"GATE exor2\t5.5\tO=!(a*b+!a*!b);\tPIN * UNKNOWN 2 999 1.9 0.5 1.9 0.5\n");
  *(undefined4 *)(param_1 + 0x1538) = 0x3fc;
  FUN_0043ebd0((uint *)(param_1 + 0x153c),(uint *)&DAT_0044ce24);
  FUN_0043ebd0((uint *)(param_1 + 0x1550),
               (uint *)
               "GATE mux2\t4\tO=!(!1D1*!3SEL+!2D2*3SEL);\nPIN\t1D1 NONINV 1 999 1 .2 1 .2\nPIN\t2D2 NONINV 1 999 1 .2 1 .2\nPIN\t3SEL UNKNOWN 1 999 1 .2 1 .2\n"
              );
  return;
}
