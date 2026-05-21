/* 0043ab65 FUN_0043ab65 */

void __fastcall FUN_0043ab65(int param_1)

{
  if (*(int *)(param_1 + 0x2c) != 0) {
    _free(*(void **)(param_1 + 0x2c));
    *(undefined4 *)(param_1 + 0x2c) = 0;
  }
  if (*(int *)(param_1 + 0x34) != 0) {
    _free(*(void **)(param_1 + 0x34));
    *(undefined4 *)(param_1 + 0x34) = 0;
  }
  return;
}
