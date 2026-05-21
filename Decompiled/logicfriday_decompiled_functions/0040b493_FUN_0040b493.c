/* 0040b493 FUN_0040b493 */

undefined4 __cdecl FUN_0040b493(void *param_1)

{
  uint local_8;
  
  for (local_8 = 0; local_8 < *(uint *)((int)param_1 + 200); local_8 = local_8 + 1) {
    if (*(int *)((int)param_1 + local_8 * 4 + 0x84) != 0) {
      _free(*(void **)((int)param_1 + local_8 * 4 + 0x84));
    }
  }
  _memset(param_1,0,0x1f0);
  return 0;
}
