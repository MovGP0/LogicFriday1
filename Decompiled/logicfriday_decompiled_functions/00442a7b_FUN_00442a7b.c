/* 00442a7b FUN_00442a7b */

int FUN_00442a7b(void)

{
  int *piVar1;
  int *piVar2;
  _ptiddata p_Var3;
  
  p_Var3 = __getptd();
  piVar1 = (int *)p_Var3->_tfpecode;
  if (piVar1 != (int *)PTR_DAT_00451fcc) {
    if (piVar1 != (int *)0x0) {
      piVar2 = (int *)piVar1[0xb];
      *piVar1 = *piVar1 + -1;
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
    }
    p_Var3->_tfpecode = (int)PTR_DAT_00451fcc;
    *(int *)PTR_DAT_00451fcc = *(int *)PTR_DAT_00451fcc + 1;
    piVar2 = *(int **)(PTR_DAT_00451fcc + 0x2c);
    if (piVar2 != (int *)0x0) {
      *piVar2 = *piVar2 + 1;
    }
    piVar2 = *(int **)(PTR_DAT_00451fcc + 0x34);
    if (piVar2 != (int *)0x0) {
      *piVar2 = *piVar2 + 1;
    }
    piVar2 = *(int **)(PTR_DAT_00451fcc + 0x30);
    if (piVar2 != (int *)0x0) {
      *piVar2 = *piVar2 + 1;
    }
    piVar2 = *(int **)(PTR_DAT_00451fcc + 0x40);
    if (piVar2 != (int *)0x0) {
      *piVar2 = *piVar2 + 1;
    }
    *(int *)(*(int *)(PTR_DAT_00451fcc + 0x4c) + 0xb4) =
         *(int *)(*(int *)(PTR_DAT_00451fcc + 0x4c) + 0xb4) + 1;
    if (((piVar1 != (int *)0x0) && (*piVar1 == 0)) && (piVar1 != &DAT_00451f78)) {
      FUN_004429b1(piVar1);
    }
  }
  return p_Var3->_tfpecode;
}
