/* 0041a38c FUN_0041a38c */

int __fastcall FUN_0041a38c(int param_1)

{
  *(undefined4 *)(param_1 + 0x10) = 0;
  *(undefined4 *)(param_1 + 0x14) = 0;
  *(undefined4 *)(param_1 + 0xb8) = 1;
  *(undefined4 *)(param_1 + 0xbc) = 0;
  *(undefined4 *)(param_1 + 0x48) = 1;
  *(undefined4 *)(param_1 + 0x4c) = 2;
  *(undefined8 *)(param_1 + 0x50) = 0x3fd999999999999a;
  SetRect((LPRECT)(param_1 + 0x28),0x2ee,0x2ee,0x2ee,0x2ee);
  SetRect((LPRECT)(param_1 + 0x38),0x2ee,0x2ee,0x2ee,0x2ee);
  return param_1;
}
