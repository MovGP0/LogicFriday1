/* 00439cf0 FUN_00439cf0 */

undefined4 __thiscall
FUN_00439cf0(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3,undefined4 param_4,
            undefined4 param_5,undefined4 param_6,undefined4 param_7,undefined4 param_8)

{
  HBRUSH pHVar1;
  
  *(undefined4 *)this = param_1;
  *(undefined4 *)((int)this + 4) = param_2;
  *(undefined4 *)((int)this + 8) = param_3;
  *(undefined4 *)((int)this + 0xc) = param_4;
  *(undefined4 *)((int)this + 0x10) = param_5;
  *(undefined4 *)((int)this + 0x18) = param_7;
  *(undefined4 *)((int)this + 0x1c) = 0;
  *(undefined4 *)((int)this + 0x20) = param_8;
  *(undefined4 *)((int)this + 0x14) = param_6;
  pHVar1 = GetSysColorBrush(10);
  *(HBRUSH *)((int)this + 0x24) = pHVar1;
  return 1;
}
