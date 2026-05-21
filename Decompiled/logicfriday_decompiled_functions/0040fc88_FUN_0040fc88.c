/* 0040fc88 FUN_0040fc88 */

void __fastcall FUN_0040fc88(int param_1)

{
  if (*(int *)(param_1 + 0x268) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x268));
  }
  return;
}
