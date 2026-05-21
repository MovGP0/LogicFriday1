/* 00444cbd FUN_00444cbd */

void FUN_00444cbd(void)

{
  if (DAT_0046c7dc == 0) {
    __lock(6);
    if (DAT_0046c7dc == 0) {
      FUN_00444709();
      DAT_0046c7dc = DAT_0046c7dc + 1;
    }
    FUN_00441cd6(6);
  }
  return;
}
