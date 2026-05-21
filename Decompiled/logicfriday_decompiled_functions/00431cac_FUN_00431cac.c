/* 00431cac FUN_00431cac */

undefined4 __fastcall FUN_00431cac(int param_1)

{
  BOOL BVar1;
  undefined4 local_1c;
  tagRECT local_18;
  int local_8;
  
  local_1c = 0;
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x16c4); local_8 = local_8 + 1) {
    if ((*(int *)(*(int *)(*(int *)(param_1 + 0x16cc) + local_8 * 4) + 0x48) == 0) &&
       (BVar1 = IntersectRect(&local_18,
                              (RECT *)(*(int *)(*(int *)(param_1 + 0x16cc) + local_8 * 4) + 200),
                              (RECT *)&stack0x00000004), BVar1 != 0)) {
      *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x16cc) + local_8 * 4) + 0xd8) = 1;
      local_1c = 1;
    }
  }
  return local_1c;
}
