/* 00416134 FUN_00416134 */

undefined4 __fastcall FUN_00416134(int param_1)

{
  int iVar1;
  bool bVar2;
  bool bVar3;
  int iVar4;
  int local_18;
  int local_8;
  
  bVar3 = true;
  iVar1 = *(int *)(*(int *)(param_1 + 8) + 0xc4);
  local_8 = 0;
  do {
    if (iVar1 <= local_8) {
      return 0;
    }
    bVar2 = false;
    for (local_18 = 0; local_18 < iVar1; local_18 = local_18 + 1) {
      iVar4 = _strcmp((char *)(*(int *)(param_1 + 8) + 0x160 + local_8 * 9),
                      (char *)(*(int *)(param_1 + 0xc) + 0x160 + local_18 * 9));
      if (iVar4 == 0) {
        bVar2 = true;
        if (local_8 != local_18) {
          bVar3 = false;
        }
        break;
      }
    }
    if (!bVar2) {
      return 2;
    }
    if (!bVar3) {
      return 1;
    }
    local_8 = local_8 + 1;
  } while( true );
}
