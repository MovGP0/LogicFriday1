/* 00421eb4 FUN_00421eb4 */

undefined4 __fastcall FUN_00421eb4(uint *param_1)

{
  void *pvVar1;
  uint local_10;
  uint local_c;
  uint local_8;
  
  for (local_8 = 0; local_8 < param_1[0x32]; local_8 = local_8 + 1) {
    pvVar1 = _realloc((void *)param_1[local_8 + 0x21],*param_1 << 2);
    param_1[local_8 + 0x21] = (uint)pvVar1;
    _memset((void *)param_1[local_8 + 0x21],0,*param_1 << 2);
    param_1[local_8 + 0x11] = 0;
    param_1[local_8 + 1] = 0;
  }
  local_8 = 0;
  do {
    if (*param_1 <= local_8) {
      for (local_8 = 0; local_8 < param_1[0x32]; local_8 = local_8 + 1) {
        if (param_1[local_8 + 0x7f] != 0) {
          _free((void *)param_1[local_8 + 0x7f]);
          param_1[local_8 + 0x7f] = 0;
        }
      }
      if (param_1[0x7e] != 0) {
        _free((void *)param_1[0x7e]);
        param_1[0x7e] = 0;
      }
      param_1[0x7d] = 0;
      return 0;
    }
    for (local_c = 0; local_c < param_1[0x32]; local_c = local_c + 1) {
      for (local_10 = 0; local_10 < param_1[0x7d]; local_10 = local_10 + 1) {
        if ((*(int *)(param_1[local_c + 0x7f] + local_10 * 4) != 0) &&
           ((local_8 | *(uint *)(param_1[0x7e] + 8 + local_10 * 0xc)) ==
            *(uint *)(param_1[0x7e] + 4 + local_10 * 0xc))) {
          *(undefined4 *)(param_1[local_c + 0x21] + local_8 * 4) = 1;
          param_1[local_c + 1] = param_1[local_c + 1] + 1;
          break;
        }
      }
    }
    local_8 = local_8 + 1;
  } while( true );
}
