/* 00445bf2 FUN_00445bf2 */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 __fastcall FUN_00445bf2(int param_1)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int local_8;
  
  local_8 = param_1;
  if (DAT_0046cd4c == 0) {
    ___initmbctable();
  }
  DAT_0046c914 = 0;
  GetModuleFileNameA((HMODULE)0x0,&DAT_0046c810,0x104);
  _DAT_0046c710 = &DAT_0046c810;
  FUN_00445a86((void *)0x0,(undefined4 *)0x0,&local_8);
  iVar1 = local_8;
  puVar2 = _malloc(param_1 + local_8 * 4);
  if (puVar2 == (undefined4 *)0x0) {
    uVar3 = 0xffffffff;
  }
  else {
    FUN_00445a86(puVar2 + iVar1,puVar2,&local_8);
    _DAT_0046c6f4 = local_8 + -1;
    uVar3 = 0;
    _DAT_0046c6f8 = puVar2;
  }
  return uVar3;
}
