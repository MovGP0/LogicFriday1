/* 00440a3e FUN_00440a3e */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __chkstk replaced with injection: alloca_probe */

void FUN_00440a3e(int param_1)

{
  DWORD DVar1;
  size_t sVar2;
  uint *_Dest;
  char *pcVar3;
  uint unaff_retaddr;
  uint local_12c [65];
  undefined1 local_28;
  uint *local_24;
  uint local_20;
  undefined1 *local_1c;
  undefined4 uStack_c;
  undefined *local_8;
  
  local_8 = &DAT_0044dd28;
  uStack_c = 0x440a4d;
  local_20 = DAT_00451a00 ^ unaff_retaddr;
  if (DAT_0046c564 == (code *)0x0) {
    if (param_1 == 1) {
      pcVar3 = "Buffer overrun detected!";
      local_24 = (uint *)0x44db90;
    }
    else {
      pcVar3 = "Unknown security failure detected!";
      local_24 = (uint *)0x44dc50;
    }
    local_28 = 0;
    DVar1 = GetModuleFileNameA((HMODULE)0x0,(LPSTR)local_12c,0x104);
    if (DVar1 == 0) {
      FUN_0043ebd0(local_12c,(uint *)"<program name unknown>");
    }
    _Dest = local_12c;
    sVar2 = _strlen((char *)local_12c);
    if (0x3c < sVar2 + 0xb) {
      sVar2 = _strlen((char *)local_12c);
      _Dest = (uint *)(&stack0xfffffea3 + sVar2);
      _strncpy((char *)_Dest,"...",3);
    }
    _strlen((char *)_Dest);
    local_1c = &stack0xfffffec8;
    FUN_0043ebd0((uint *)&stack0xfffffec8,(uint *)pcVar3);
    FUN_0043ebe0((uint *)&stack0xfffffec8,(uint *)&DAT_0044aad8);
    FUN_0043ebe0((uint *)&stack0xfffffec8,(uint *)"Program: ");
    FUN_0043ebe0((uint *)&stack0xfffffec8,_Dest);
    FUN_0043ebe0((uint *)&stack0xfffffec8,(uint *)&DAT_0044aad8);
    FUN_0043ebe0((uint *)&stack0xfffffec8,local_24);
    ___crtMessageBoxA(&stack0xfffffec8,"Microsoft Visual C++ Runtime Library",0x12010);
  }
  else {
    local_8 = (undefined *)0x0;
    (*DAT_0046c564)();
    local_8 = (undefined *)0xffffffff;
  }
                    /* WARNING: Subroutine does not return */
  __exit(3);
}
