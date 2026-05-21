/* 0040d301 FUN_0040d301 */

char * __cdecl FUN_0040d301(int param_1)

{
  int local_8;
  
  local_8 = 0;
  while( true ) {
    if (DAT_004519a0 <= local_8) {
      return (char *)0x0;
    }
    if (param_1 == *(int *)(&DAT_004516a4 + local_8 * 0x30)) break;
    local_8 = local_8 + 1;
  }
  return s_New____004516aa + local_8 * 0x30;
}
