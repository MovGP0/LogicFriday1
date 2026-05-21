/* 00429b01 FUN_00429b01 */

void FUN_00429b01(void)

{
  int *piVar1;
  HGDIOBJ pvVar2;
  void *pvVar3;
  undefined4 *puVar4;
  undefined4 extraout_ECX;
  int iVar5;
  int iVar6;
  int unaff_EBP;
  undefined4 *puVar7;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x20) = extraout_ECX;
  pvVar2 = SelectObject(*(HDC *)(*(int *)(unaff_EBP + -0x20) + 0x2318),
                        *(HGDIOBJ *)(*(int *)(unaff_EBP + -0x20) + 9000));
  *(HGDIOBJ *)(unaff_EBP + -0x10) = pvVar2;
  pvVar3 = operator_new(0x50);
  *(void **)(unaff_EBP + -0x1c) = pvVar3;
  *(undefined4 *)(unaff_EBP + -4) = 0;
  if (*(int *)(unaff_EBP + -0x1c) == 0) {
    *(undefined4 *)(unaff_EBP + -0x24) = 0;
  }
  else {
    puVar4 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x1c));
    *(undefined4 **)(unaff_EBP + -0x24) = puVar4;
  }
  *(undefined4 *)(unaff_EBP + -0x18) = *(undefined4 *)(unaff_EBP + -0x24);
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  *(undefined4 *)
   (*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
   *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) = *(undefined4 *)(unaff_EBP + -0x18);
  puVar4 = *(undefined4 **)(unaff_EBP + 8);
  puVar7 = *(undefined4 **)
            (*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
            *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4);
  for (iVar5 = 0x14; iVar5 != 0; iVar5 = iVar5 + -1) {
    *puVar7 = *puVar4;
    puVar4 = puVar4 + 1;
    puVar7 = puVar7 + 1;
  }
  *(undefined4 *)
   (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
            *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x4c) =
       *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8);
  MoveToEx(*(HDC *)(*(int *)(unaff_EBP + -0x20) + 0x2318),
           **(int **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                              *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c),
           *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c) + 4
                   ),(LPPOINT)0x0);
  if (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) == 2) {
    piVar1 = *(int **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                               *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c);
    FUN_0043ae30(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                           *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                            *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                   0xc) * 4),*piVar1,piVar1[1],
                 *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8),0,
                 *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                           *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                  0x2c) + 0xc));
    FUN_004287c6(*(void **)(unaff_EBP + -0x20),*(HDC *)(*(int *)(unaff_EBP + -0x20) + 0x2318),
                 **(int **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                    *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c),
                 *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                           *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                  0x2c) + 4));
    if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                         *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x38) != -3) {
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                 *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0xc) * 4) +
       0x38) = *(undefined4 *)
                (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                         *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x38);
    }
  }
  *(undefined4 *)(unaff_EBP + -0x14) = 1;
  while (*(int *)(unaff_EBP + -0x14) <
         *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                          *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x28)) {
    LineTo(*(HDC *)(*(int *)(unaff_EBP + -0x20) + 0x2318),
           *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c) +
                   *(int *)(unaff_EBP + -0x14) * 0x14),
           *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c) + 4
                   + *(int *)(unaff_EBP + -0x14) * 0x14));
    *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
  }
  if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                       *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x14) == 2) {
    iVar6 = (*(int *)(unaff_EBP + -0x14) + -1) * 0x14;
    iVar5 = *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                             *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x2c);
    FUN_0043ae30(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                           *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                            *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                   0x20) * 4),*(int *)(iVar5 + iVar6),*(int *)(iVar5 + 4 + iVar6),
                 *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8),
                 *(int *)(unaff_EBP + -0x14) + -1,
                 *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                           *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                  0x2c) + 0xc + (*(int *)(unaff_EBP + -0x14) + -2) * 0x14));
    FUN_004287c6(*(void **)(unaff_EBP + -0x20),*(HDC *)(*(int *)(unaff_EBP + -0x20) + 0x2318),
                 *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                           *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                  0x2c) + (*(int *)(unaff_EBP + -0x14) + -1) * 0x14),
                 *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                           *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) +
                                  0x2c) + 4 + (*(int *)(unaff_EBP + -0x14) + -1) * 0x14));
    if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                         *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x38) != -3) {
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                                 *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x20) * 4) +
       0x38) = *(undefined4 *)
                (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16d0) +
                         *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) * 4) + 0x38);
    }
  }
  if (**(int **)(unaff_EBP + 8) == 1) {
    *(undefined4 *)
     (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16cc) +
              *(int *)(*(int *)(unaff_EBP + 8) + 4) * 4) + 0xe0) =
         *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8);
  }
  else if (**(int **)(unaff_EBP + 8) == 0) {
    *(undefined4 *)
     (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16cc) +
              *(int *)(*(int *)(unaff_EBP + 8) + 4) * 4) + 0xe4 +
     *(int *)(*(int *)(unaff_EBP + 8) + 8) * 4) =
         *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8);
  }
  if (*(int *)(*(int *)(unaff_EBP + 8) + 0x14) == 1) {
    *(undefined4 *)
     (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16cc) +
              *(int *)(*(int *)(unaff_EBP + 8) + 0x18) * 4) + 0xe0) =
         *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8);
  }
  else if (*(int *)(*(int *)(unaff_EBP + 8) + 0x14) == 0) {
    *(undefined4 *)
     (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x20) + 0x16cc) +
              *(int *)(*(int *)(unaff_EBP + 8) + 0x18) * 4) + 0xe4 +
     *(int *)(*(int *)(unaff_EBP + 8) + 0x1c) * 4) =
         *(undefined4 *)(*(int *)(unaff_EBP + -0x20) + 0x16c8);
  }
  *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) =
       *(int *)(*(int *)(unaff_EBP + -0x20) + 0x16c8) + 1;
  SelectObject(*(HDC *)(*(int *)(unaff_EBP + -0x20) + 0x2318),*(HGDIOBJ *)(unaff_EBP + -0x10));
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return;
}
