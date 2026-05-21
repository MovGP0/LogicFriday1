/* 0040ae7a FUN_0040ae7a */

undefined4 FUN_0040ae7a(HWND param_1,int param_2,ushort param_3,void *param_4)

{
  undefined4 uVar1;
  
  if (param_2 == 0x8008) {
    DAT_0046c4e8 = param_4;
    uVar1 = 1;
  }
  else if (DAT_0046c4e8 == (void *)0x0) {
    uVar1 = 1;
  }
  else {
    uVar1 = FUN_00436c32(DAT_0046c4e8,param_1,param_2,param_3);
  }
  return uVar1;
}
