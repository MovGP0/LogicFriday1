/* 0040c130 FUN_0040c130 */

void __cdecl FUN_0040c130(int param_1)

{
  DAT_00452ed4 = 1;
  FUN_0040bdc3(0x44abdc);
  FUN_0043983d(&DAT_00453e28,*(LPARAM *)(param_1 + 0x108));
  if (DAT_00452eb4 == 0) {
    FUN_0040bdc3(0x44ad68);
  }
  else {
    FUN_0040bdc3(0x44a840);
  }
  DAT_00452ed4 = 0;
  FUN_0040e77d();
  FUN_0043ea5f();
  return;
}
