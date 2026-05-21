/* 0043f596 __onexit_lk */

/* Library Function - Single Match
    __onexit_lk
   
   Library: Visual Studio 2003 Release */

void __onexit_lk(void)

{
  size_t sVar1;
  void *pvVar2;
  size_t sVar3;
  undefined4 unaff_EDI;
  
  sVar1 = __msize(DAT_0046cd48);
  if (sVar1 < (uint)((int)DAT_0046cd44 + (4 - (int)DAT_0046cd48))) {
    sVar3 = 0x800;
    if (sVar1 < 0x800) {
      sVar3 = sVar1;
    }
    pvVar2 = _realloc(DAT_0046cd48,sVar3 + sVar1);
    if (pvVar2 == (void *)0x0) {
      pvVar2 = _realloc(DAT_0046cd48,sVar1 + 0x10);
      if (pvVar2 == (void *)0x0) {
        return;
      }
    }
    DAT_0046cd44 = (undefined4 *)((int)pvVar2 + ((int)DAT_0046cd44 - (int)DAT_0046cd48 >> 2) * 4);
    DAT_0046cd48 = pvVar2;
  }
  *DAT_0046cd44 = unaff_EDI;
  DAT_0046cd44 = DAT_0046cd44 + 1;
  return;
}
