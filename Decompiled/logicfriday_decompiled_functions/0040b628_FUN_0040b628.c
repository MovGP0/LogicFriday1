/* 0040b628 FUN_0040b628 */

undefined4 FUN_0040b628(void)

{
  undefined4 uVar1;
  void *pvVar2;
  int unaff_EBP;
  
  FUN_0043f30c();
  *(int *)(unaff_EBP + -0x10) = DAT_004528a0;
  DAT_004528a0 = DAT_004528a0 + 1;
  DAT_004528a4 = _realloc(DAT_004528a4,DAT_004528a0 * 0x118);
  if (DAT_004528a4 == (void *)0x0) {
    uVar1 = 0xffffffff;
  }
  else {
    pvVar2 = operator_new(0x2700);
    *(void **)(unaff_EBP + -0x18) = pvVar2;
    *(undefined4 *)(unaff_EBP + -4) = 0;
    if (*(int *)(unaff_EBP + -0x18) == 0) {
      *(undefined4 *)(unaff_EBP + -0x1c) = 0;
    }
    else {
      uVar1 = FUN_0041cbd4();
      *(undefined4 *)(unaff_EBP + -0x1c) = uVar1;
    }
    *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x1c);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    *(undefined4 *)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118 + 0x110) =
         *(undefined4 *)(unaff_EBP + -0x14);
    if (*(int *)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118 + 0x110) == 0) {
      uVar1 = 0xffffffff;
    }
    else {
      FUN_0043ebd0((uint *)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118 + 9),
                   (uint *)&DAT_0044ad26);
      FUN_0043ebd0((uint *)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118),
                   (uint *)&DAT_0044ad26);
      *(undefined4 *)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118 + 0x114) =
           *(undefined4 *)
            (*(int *)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118 + 0x110) + 0x23c);
      FUN_0041def5(*(void **)((int)DAT_004528a4 + *(int *)(unaff_EBP + -0x10) * 0x118 + 0x110),
                   *(undefined4 *)(unaff_EBP + 8));
      uVar1 = *(undefined4 *)(unaff_EBP + -0x10);
    }
  }
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return uVar1;
}
