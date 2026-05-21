/* 004484a8 FUN_004484a8 */

undefined4 __cdecl FUN_004484a8(byte param_1,uint param_2,byte param_3)

{
  uint uVar1;
  
  if (((&DAT_0046ca01)[param_1] & param_3) == 0) {
    if (param_2 == 0) {
      uVar1 = 0;
    }
    else {
      uVar1 = *(ushort *)(&DAT_0044e50a + (uint)param_1 * 2) & param_2;
    }
    if (uVar1 == 0) {
      return 0;
    }
  }
  return 1;
}
