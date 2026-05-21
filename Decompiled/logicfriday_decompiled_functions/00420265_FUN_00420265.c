/* 00420265 FUN_00420265 */

int __fastcall FUN_00420265(uint *param_1)

{
  bool bVar1;
  uint local_14;
  uint local_c;
  uint local_8;
  
  local_8 = 0;
  do {
    if (*param_1 <= local_8) {
      return 0;
    }
    for (local_c = 0; local_c < param_1[0x32]; local_c = local_c + 1) {
      if (*(int *)(param_1[local_c + 0x21] + local_8 * 4) != 2) {
        bVar1 = false;
        for (local_14 = 0; local_14 < param_1[0x7d]; local_14 = local_14 + 1) {
          if ((*(int *)(param_1[local_c + 0x7f] + local_14 * 4) != 0) &&
             ((local_8 | *(uint *)(param_1[0x7e] + 8 + local_14 * 0xc)) ==
              *(uint *)(param_1[0x7e] + 4 + local_14 * 0xc))) {
            bVar1 = true;
            break;
          }
        }
        if ((bVar1) && (*(int *)(param_1[local_c + 0x21] + local_8 * 4) == 0)) {
          return local_8 + 0xc0000;
        }
        if ((!bVar1) && (*(int *)(param_1[local_c + 0x21] + local_8 * 4) != 0)) {
          return local_8 + 0xb0000;
        }
      }
    }
    local_8 = local_8 + 1;
  } while( true );
}
