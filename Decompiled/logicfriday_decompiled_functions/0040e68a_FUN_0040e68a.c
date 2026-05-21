/* 0040e68a FUN_0040e68a */

undefined4 __cdecl FUN_0040e68a(LPCSTR param_1)

{
  int iVar1;
  undefined4 uVar2;
  
  if (DAT_00452e90 == 0) {
    if (DAT_00452e98 == 0) {
      if (DAT_00452e94 == 0) {
        uVar2 = 1;
      }
      else {
        iVar1 = MessageBoxA(DAT_00452aac,"Do you want to cancel diagram entry?",param_1,0x104);
        if (iVar1 == 6) {
          SendMessageA(DAT_00452aac,0x111,0x158,0);
          uVar2 = 1;
        }
        else {
          uVar2 = 0;
        }
      }
    }
    else {
      iVar1 = MessageBoxA(DAT_00452aac,"Do you want to cancel equation entry?",param_1,0x104);
      if (iVar1 == 6) {
        SendMessageA(DAT_00452aac,0x111,0x150,0);
        uVar2 = 1;
      }
      else {
        uVar2 = 0;
      }
    }
  }
  else {
    iVar1 = MessageBoxA(DAT_00452aac,"Do you want to cancel truth table entry?",param_1,0x104);
    if (iVar1 == 6) {
      SendMessageA(DAT_00452aac,0x111,0xe6,0);
      uVar2 = 1;
    }
    else {
      uVar2 = 0;
    }
  }
  return uVar2;
}
