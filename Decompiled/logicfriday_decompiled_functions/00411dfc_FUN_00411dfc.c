/* 00411dfc FUN_00411dfc */

undefined4 FUN_00411dfc(int param_1,int param_2,int param_3)

{
  int iVar1;
  int iVar2;
  int local_8;
  
  iVar1 = *(int *)(param_1 + 200);
  if (*(int *)(*(int *)(param_1 + 0x84) + param_2 * 4) == -1) {
    for (local_8 = 0; local_8 < iVar1; local_8 = local_8 + 1) {
      if (*(char *)(param_3 + local_8) == '1') {
        *(undefined4 *)(*(int *)(param_1 + 0x84 + local_8 * 4) + param_2 * 4) = 1;
        *(int *)(param_1 + 4 + local_8 * 4) = *(int *)(param_1 + 4 + local_8 * 4) + 1;
      }
      else if (*(char *)(param_3 + local_8) == '0') {
        *(undefined4 *)(*(int *)(param_1 + 0x84 + local_8 * 4) + param_2 * 4) = 0;
      }
      else {
        if (*(char *)(param_3 + local_8) != 'X') {
          return 1;
        }
        *(undefined4 *)(*(int *)(param_1 + 0x84 + local_8 * 4) + param_2 * 4) = 2;
        *(int *)(param_1 + 0x44 + local_8 * 4) = *(int *)(param_1 + 0x44 + local_8 * 4) + 1;
      }
    }
  }
  else {
    for (local_8 = 0; local_8 < iVar1; local_8 = local_8 + 1) {
      iVar2 = *(int *)(*(int *)(param_1 + 0x84 + local_8 * 4) + param_2 * 4);
      if ((*(char *)(param_3 + local_8) == '1') && (iVar2 != 1)) {
        return 10;
      }
      if ((*(char *)(param_3 + local_8) == '0') && (iVar2 != 0)) {
        return 10;
      }
      if ((*(char *)(param_3 + local_8) == 'X') && (iVar2 != 2)) {
        return 10;
      }
    }
  }
  return 0;
}
