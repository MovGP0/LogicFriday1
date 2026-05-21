/* 00446edc ___free_lconv_mon */

/* Library Function - Single Match
    ___free_lconv_mon
   
   Library: Visual Studio 2003 Release */

void __cdecl ___free_lconv_mon(int param_1)

{
  undefined *puVar1;
  
  if (param_1 != 0) {
    puVar1 = *(undefined **)(param_1 + 0xc);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0xc)) && (puVar1 != PTR_DAT_004523f0)) {
      _free(puVar1);
    }
    puVar1 = *(undefined **)(param_1 + 0x10);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0x10)) && (puVar1 != PTR_DAT_004523f4)) {
      _free(puVar1);
    }
    puVar1 = *(undefined **)(param_1 + 0x14);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0x14)) && (puVar1 != PTR_DAT_004523f8)) {
      _free(puVar1);
    }
    puVar1 = *(undefined **)(param_1 + 0x18);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0x18)) && (puVar1 != PTR_DAT_004523fc)) {
      _free(puVar1);
    }
    puVar1 = *(undefined **)(param_1 + 0x1c);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0x1c)) && (puVar1 != PTR_DAT_00452400)) {
      _free(puVar1);
    }
    puVar1 = *(undefined **)(param_1 + 0x20);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0x20)) && (puVar1 != PTR_DAT_00452404)) {
      _free(puVar1);
    }
    puVar1 = *(undefined **)(param_1 + 0x24);
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 0x24)) && (puVar1 != PTR_DAT_00452408)) {
      _free(puVar1);
    }
  }
  return;
}
