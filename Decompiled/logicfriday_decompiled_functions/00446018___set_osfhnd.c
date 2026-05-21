/* 00446018 __set_osfhnd */

/* Library Function - Single Match
    __set_osfhnd
   
   Library: Visual Studio 2003 Release */

int __cdecl __set_osfhnd(int param_1,intptr_t param_2)

{
  int *piVar1;
  ulong *puVar2;
  int iVar3;
  DWORD nStdHandle;
  
  if ((uint)param_1 < DAT_0046cc2c) {
    iVar3 = (param_1 & 0x1fU) * 0x24;
    if (*(int *)(iVar3 + (&DAT_0046cc40)[param_1 >> 5]) == -1) {
      if (DAT_00451a44 == 1) {
        if (param_1 == 0) {
          nStdHandle = 0xfffffff6;
        }
        else if (param_1 == 1) {
          nStdHandle = 0xfffffff5;
        }
        else {
          if (param_1 != 2) goto LAB_00446071;
          nStdHandle = 0xfffffff4;
        }
        SetStdHandle(nStdHandle,(HANDLE)param_2);
      }
LAB_00446071:
      *(intptr_t *)(iVar3 + (&DAT_0046cc40)[param_1 >> 5]) = param_2;
      return 0;
    }
  }
  piVar1 = FUN_00441a24();
  *piVar1 = 9;
  puVar2 = FUN_00441a2d();
  *puVar2 = 0;
  return -1;
}
