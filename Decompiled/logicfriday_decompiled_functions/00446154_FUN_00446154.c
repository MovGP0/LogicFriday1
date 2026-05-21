/* 00446154 FUN_00446154 */

undefined4 __cdecl FUN_00446154(uint param_1)

{
  int iVar1;
  int iVar2;
  int iVar3;
  
  iVar3 = (param_1 & 0x1f) * 0x24;
  iVar2 = (&DAT_0046cc40)[(int)param_1 >> 5] + iVar3;
  if (*(int *)(iVar2 + 8) == 0) {
    __lock(10);
    if (*(int *)(iVar2 + 8) == 0) {
      iVar1 = ___crtInitCritSecAndSpinCount(iVar2 + 0xc,4000);
      if (iVar1 == 0) {
        FUN_00441cd6(10);
        return 0;
      }
      *(int *)(iVar2 + 8) = *(int *)(iVar2 + 8) + 1;
    }
    FUN_00441cd6(10);
  }
  EnterCriticalSection((LPCRITICAL_SECTION)((&DAT_0046cc40)[(int)param_1 >> 5] + 0xc + iVar3));
  return 1;
}
