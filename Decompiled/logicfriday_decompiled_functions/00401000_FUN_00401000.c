/* 00401000 FUN_00401000 */

undefined4 __cdecl FUN_00401000(int param_1)

{
  uint local_8;
  
  local_8 = 0;
  while( true ) {
    if (1 < local_8) {
      return 0;
    }
    if ((&DAT_00451060)[local_8 * 0xc6] == param_1) break;
    local_8 = local_8 + 1;
  }
  return 1;
}
