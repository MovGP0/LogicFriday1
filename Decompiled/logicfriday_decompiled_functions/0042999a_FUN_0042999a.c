/* 0042999a FUN_0042999a */

void FUN_0042999a(void)

{
  void *pvVar1;
  int iVar2;
  undefined4 extraout_ECX;
  int unaff_EBP;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x1c) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x1c) + 0x16c4);
  pvVar1 = operator_new(0xfc);
  *(void **)(unaff_EBP + -0x18) = pvVar1;
  *(undefined4 *)(unaff_EBP + -4) = 0;
  if (*(int *)(unaff_EBP + -0x18) == 0) {
    *(undefined4 *)(unaff_EBP + -0x20) = 0;
  }
  else {
    iVar2 = FUN_004175df(*(int *)(unaff_EBP + -0x18));
    *(int *)(unaff_EBP + -0x20) = iVar2;
  }
  *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x20);
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4)
       = *(undefined4 *)(unaff_EBP + -0x14);
  if ((((*(int *)(unaff_EBP + 0x10) != 0x438) && (*(int *)(unaff_EBP + 0x10) != 0x439)) &&
      (*(int *)(unaff_EBP + 0x10) != 0x42a)) && (*(int *)(unaff_EBP + 0x10) != 0x408)) {
    FUN_0043ed39((char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16cc) +
                                  *(int *)(unaff_EBP + -0x10) * 4) + 0x50),&DAT_0044cbbc);
    FUN_0043ed39((char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16cc) +
                                  *(int *)(unaff_EBP + -0x10) * 4) + 4),&DAT_0044cbbc);
    *(int *)(*(int *)(unaff_EBP + -0x1c) + 0x234c) =
         *(int *)(*(int *)(unaff_EBP + -0x1c) + 0x234c) + 1;
  }
  *(undefined4 *)
   (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4) +
   0xb4) = 0;
  FUN_0042af77(*(undefined4 **)
                (*(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4),
               *(undefined4 *)(unaff_EBP + 0x10));
  FUN_00425f03(*(HDC *)(*(int *)(unaff_EBP + -0x1c) + 0x2318),
               *(int **)(*(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16cc) +
                        *(int *)(unaff_EBP + -0x10) * 4),*(int *)(unaff_EBP + 8),
               *(int *)(unaff_EBP + 0xc),1);
  *(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16c4) =
       *(int *)(*(int *)(unaff_EBP + -0x1c) + 0x16c4) + 1;
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return;
}
