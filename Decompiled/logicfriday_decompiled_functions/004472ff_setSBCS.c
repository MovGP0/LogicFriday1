/* 004472ff setSBCS */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* Library Function - Single Match
    _setSBCS
   
   Library: Visual Studio 2003 Release */

void __cdecl setSBCS(void)

{
  int iVar1;
  undefined4 *puVar2;
  
  puVar2 = (undefined4 *)&DAT_0046ca00;
  for (iVar1 = 0x40; iVar1 != 0; iVar1 = iVar1 + -1) {
    *puVar2 = 0;
    puVar2 = puVar2 + 1;
  }
  *(undefined1 *)puVar2 = 0;
  DAT_0046cb04 = 0;
  DAT_0046c9fc = 0;
  DAT_0046c9f4 = 0;
  _DAT_0046cb10 = 0;
  DAT_0046cb14 = 0;
  DAT_0046cb18 = 0;
  return;
}
