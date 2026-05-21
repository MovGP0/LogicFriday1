/* 00442c1a FUN_00442c1a */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */

void __cdecl FUN_00442c1a(LPVOID param_1)

{
  int *piVar1;
  int *piVar2;
  
  if (DAT_00452104 != 0xffffffff) {
    if ((param_1 != (LPVOID)0x0) || (param_1 = TlsGetValue(DAT_00452104), param_1 != (LPVOID)0x0)) {
      if (*(void **)((int)param_1 + 0x24) != (void *)0x0) {
        _free(*(void **)((int)param_1 + 0x24));
      }
      if (*(void **)((int)param_1 + 0x2c) != (void *)0x0) {
        _free(*(void **)((int)param_1 + 0x2c));
      }
      if (*(void **)((int)param_1 + 0x34) != (void *)0x0) {
        _free(*(void **)((int)param_1 + 0x34));
      }
      if (*(void **)((int)param_1 + 0x3c) != (void *)0x0) {
        _free(*(void **)((int)param_1 + 0x3c));
      }
      if (*(void **)((int)param_1 + 0x44) != (void *)0x0) {
        _free(*(void **)((int)param_1 + 0x44));
      }
      if (*(void **)((int)param_1 + 0x48) != (void *)0x0) {
        _free(*(void **)((int)param_1 + 0x48));
      }
      if (*(undefined **)((int)param_1 + 0x54) != &DAT_00452108) {
        _free(*(undefined **)((int)param_1 + 0x54));
      }
      __lock(0xd);
      piVar1 = *(int **)((int)param_1 + 0x60);
      if (((piVar1 != (int *)0x0) && (*piVar1 = *piVar1 + -1, *piVar1 == 0)) &&
         (piVar1 != DAT_0046c9f8)) {
        _free(piVar1);
      }
      FUN_00442d70();
      __lock(0xc);
      piVar1 = *(int **)((int)param_1 + 100);
      if (piVar1 != (int *)0x0) {
        *piVar1 = *piVar1 + -1;
        piVar2 = (int *)piVar1[0xb];
        if (piVar2 != (int *)0x0) {
          *piVar2 = *piVar2 + -1;
        }
        piVar2 = (int *)piVar1[0xd];
        if (piVar2 != (int *)0x0) {
          *piVar2 = *piVar2 + -1;
        }
        piVar2 = (int *)piVar1[0xc];
        if (piVar2 != (int *)0x0) {
          *piVar2 = *piVar2 + -1;
        }
        piVar2 = (int *)piVar1[0x10];
        if (piVar2 != (int *)0x0) {
          *piVar2 = *piVar2 + -1;
        }
        *(int *)(piVar1[0x13] + 0xb4) = *(int *)(piVar1[0x13] + 0xb4) + -1;
        if (((piVar1 != (int *)PTR_DAT_00451fcc) && (piVar1 != &DAT_00451f78)) && (*piVar1 == 0)) {
          FUN_004429b1(piVar1);
        }
      }
      FUN_00442d7c();
      _free(param_1);
    }
    TlsSetValue(DAT_00452104,(LPVOID)0x0);
    return;
  }
  return;
}
