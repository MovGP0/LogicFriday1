/* 0040aebc FUN_0040aebc */

undefined4 FUN_0040aebc(HWND param_1,int param_2,ushort param_3,void *param_4)

{
  undefined4 uVar1;
  
  if (param_2 == 0x8008) {
    DAT_0046c4ec = param_4;
    uVar1 = 1;
  }
  else if (DAT_0046c4ec == (void *)0x0) {
    uVar1 = 1;
  }
  else {
    uVar1 = FUN_0042424d(DAT_0046c4ec,param_1,param_2,param_3);
  }
  return uVar1;
}
