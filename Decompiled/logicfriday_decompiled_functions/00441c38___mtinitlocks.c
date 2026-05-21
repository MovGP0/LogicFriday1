/* 00441c38 __mtinitlocks */

/* Library Function - Single Match
    __mtinitlocks
   
   Library: Visual Studio 2003 Release */

int __cdecl __mtinitlocks(void)

{
  int iVar1;
  int iVar2;
  undefined *puVar3;
  
  iVar2 = 0;
  puVar3 = &DAT_0046c578;
  do {
    if ((&DAT_00451e44)[iVar2 * 2] == 1) {
      (&DAT_00451e40)[iVar2 * 2] = puVar3;
      puVar3 = puVar3 + 0x18;
      iVar1 = ___crtInitCritSecAndSpinCount((&DAT_00451e40)[iVar2 * 2],4000);
      if (iVar1 == 0) {
        (&DAT_00451e40)[iVar2 * 2] = 0;
        return 0;
      }
    }
    iVar2 = iVar2 + 1;
  } while (iVar2 < 0x24);
  return 1;
}
