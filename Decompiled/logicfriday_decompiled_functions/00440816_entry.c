/* 00440816 entry */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

WPARAM entry(void)

{
  HMODULE pHVar1;
  int iVar2;
  byte *pbVar3;
  WPARAM WVar4;
  int *piVar5;
  int extraout_ECX;
  undefined4 uVar6;
  _OSVERSIONINFOA local_114;
  _STARTUPINFOA local_68;
  int local_24;
  uint local_20;
  undefined1 *local_1c;
  undefined4 uStack_c;
  undefined *local_8;
  
  local_8 = &DAT_0044db30;
  uStack_c = 0x440822;
  builtin_memcpy(local_114.szCSDVersion + 0x7c,".\bD",4);
  local_1c = (undefined1 *)&local_114;
  local_114.dwOSVersionInfoSize = 0x94;
  GetVersionExA(&local_114);
  DAT_0046c6e0 = local_114.dwPlatformId;
  DAT_0046c6ec = local_114.dwMajorVersion;
  _DAT_0046c6f0 = local_114.dwMinorVersion;
  _DAT_0046c6e4 = local_114.dwBuildNumber & 0x7fff;
  if (local_114.dwPlatformId != 2) {
    _DAT_0046c6e4 = _DAT_0046c6e4 | 0x8000;
  }
  _DAT_0046c6e8 = local_114.dwMajorVersion * 0x100 + local_114.dwMinorVersion;
  pHVar1 = GetModuleHandleA((LPCSTR)0x0);
  if (((short)pHVar1->unused == 0x5a4d) &&
     (piVar5 = (int *)((int)&pHVar1->unused + pHVar1[0xf].unused), *piVar5 == 0x4550)) {
    if ((short)piVar5[6] == 0x10b) {
      if (0xe < (uint)piVar5[0x1d]) {
        iVar2 = piVar5[0x3a];
        goto LAB_004408d7;
      }
    }
    else if (((short)piVar5[6] == 0x20b) && (0xe < (uint)piVar5[0x21])) {
      iVar2 = piVar5[0x3e];
LAB_004408d7:
      local_20 = (uint)(iVar2 != 0);
      goto LAB_004408dd;
    }
  }
  local_20 = 0;
LAB_004408dd:
  iVar2 = __heap_init();
  if (iVar2 == 0) {
    fast_error_exit(0x1c);
  }
  iVar2 = FUN_00442d85();
  if (iVar2 == 0) {
    fast_error_exit(0x10);
  }
  FUN_00445db6();
  local_8 = (undefined *)0x0;
  iVar2 = FUN_004451ec();
  if (iVar2 < 0) {
    __amsg_exit(0x1b);
  }
  DAT_0046dd84 = GetCommandLineA();
  DAT_0046c558 = ___crtGetEnvironmentStringsA();
  iVar2 = FUN_00445bf2(extraout_ECX);
  if (iVar2 < 0) {
    __amsg_exit(8);
  }
  iVar2 = __setenvp();
  if (iVar2 < 0) {
    __amsg_exit(9);
  }
  local_24 = FUN_00442e36();
  if (local_24 != 0) {
    __amsg_exit(local_24);
  }
  local_68.dwFlags = 0;
  GetStartupInfoA(&local_68);
  pbVar3 = FUN_00445956();
  uVar6 = 0;
  pHVar1 = GetModuleHandleA((LPCSTR)0x0);
  WVar4 = FUN_0040145f(pHVar1,uVar6,(char *)pbVar3);
  if (local_20 == 0) {
    FUN_00442f6e(WVar4);
  }
  FUN_00442f90();
  return WVar4;
}
