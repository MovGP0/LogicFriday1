/* 00444e12 __fseeki64_lk */

/* Library Function - Single Match
    __fseeki64_lk
   
   Library: Visual Studio 2003 Release */

undefined4 __cdecl __fseeki64_lk(FILE *param_1,undefined4 param_2,undefined4 param_3,int param_4)

{
  uint uVar1;
  int *piVar2;
  int unaff_EDI;
  ulonglong uVar3;
  longlong lVar4;
  
  lVar4 = CONCAT44(param_3,param_2);
  if (((param_1->_flag & 0x83U) == 0) || (((param_4 != 0 && (param_4 != 1)) && (param_4 != 2)))) {
    piVar2 = FUN_00441a24();
    *piVar2 = 0x16;
  }
  else {
    param_1->_flag = param_1->_flag & 0xffffffef;
    if (param_4 == 1) {
      uVar3 = __ftelli64_lk((uint *)param_1);
      lVar4 = uVar3 + lVar4;
      param_4 = 0;
    }
    param_3 = (undefined4)((ulonglong)lVar4 >> 0x20);
    __flush(param_1);
    uVar1 = param_1->_flag;
    if ((char)uVar1 < '\0') {
      param_1->_flag = uVar1 & 0xfffffffc;
    }
    else if ((((uVar1 & 1) != 0) && ((uVar1 & 8) != 0)) && ((uVar1 & 0x400) == 0)) {
      param_1->_bufsiz = 0x200;
    }
    lVar4 = __lseeki64(param_1->_file,CONCAT44(param_4,param_3),unaff_EDI);
    if (lVar4 != -1) {
      return 0;
    }
  }
  return 0xffffffff;
}
