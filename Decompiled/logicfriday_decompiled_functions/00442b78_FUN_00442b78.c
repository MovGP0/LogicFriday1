/* 00442b78 FUN_00442b78 */

void FUN_00442b78(void)

{
  __mtdeletelocks();
  if (DAT_00452104 != 0xffffffff) {
    TlsFree(DAT_00452104);
    DAT_00452104 = 0xffffffff;
  }
  return;
}
